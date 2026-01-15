use std::sync::{Arc, LazyLock, Mutex};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

/// New handlers that will be added by the ctrlc thread
/// once ctrlc is pressed
static CTRLC_HANDLERS_NEW: Mutex<Vec<Box<dyn FnMut() +Send>>> = Mutex::new(Vec::new());
/// Stack of CtrlC frames to signal
static CTRLC_SIGNAL_STACK: Mutex<Vec<CtrlcSignal>> = Mutex::new(Vec::new());
/// Thread safe init lock
static INIT_ONCE: LazyLock<Result<(), String>> = LazyLock::new(|| {
    let mut handlers = vec![];
    let set_result = ctrlc::try_set_handler(move || {
        // populate new handlers
        if let Ok(mut new_handlers) = CTRLC_HANDLERS_NEW.lock() {
            handlers.extend(new_handlers.drain(..));
        }
        // signal the stack
        if let Ok(stack) = CTRLC_SIGNAL_STACK.lock() {
            if let Some(frame) = stack.last() {
                frame.signal();
            }
        }

        // note we are not holding any lock when invoking user-defined handlers
        for handler in handlers.iter_mut().rev() {
            handler();
        }

    }); 
    match set_result {
        Err(ctrlc::Error::MultipleHandlers) => {
            Err("failed to set ctrl-c handler: a handler is already set using the `ctrlc` crate. please set with cu::cli instead (see documentation for more information)".to_string())
        }
        Err(other_error) => {
            Err(format!("failed to set ctrl-c handler: {other_error}"))
        }
        Ok(_) => Ok(())
    }
});

/// Add a global handler to handle Ctrl-C signals
///
///
///
#[cfg(feature = "cli")]
pub fn add_global_ctrlc_handler<F: FnMut() + Send + 'static>(handler: F) -> cu::Result<()> {
    {
        let Ok(mut handlers) = CTRLC_HANDLERS_NEW.lock() else {
            cu::bail!("global ctrl-c handler vector is poisoned");
        };
        handlers.push(Box::new(handler));
    }
    if let Err(e) = &*INIT_ONCE {
        cu::bail!("{e}");
    }
    Ok(())
}

/// # Handling Ctrl-C
///
/// The [`ctrlc`](https://docs.rs/ctrlc) crate provides a low-level cross-platform
/// way to set up a handler for `Ctrl-C`. `cu` builds wrappers around it to provide
/// a better experience handling user interrupts.
///
/// # Action Frames
/// Most of the time, custom Ctrl-C behavior is for executing some long running
/// tasks and give the user the ability to abort it. `cu` maintains a stack
/// of these "action frames" that are created using `cu::cli::catch_ctrlc`
/// or the async version [`cu::cli::co_catch_ctrlc`]. The sync and async versions
/// have slightly different behavior, but the spirit is the same:
///
/// - The action is executed on a different thread (or asynchronously, in the async version)
/// - When user hits `Ctrl-C`, the top-most action frame on the stack is signalled
/// - The calling thread/task polls for the `Ctrl-C` signal, if signalled,
///   it will return.
/// - The action frame is popped when the 
///
/// The return value of the inner `cu::Result<T>` will be transformed into
/// `cu::Result<Option<T>>`, where:
/// - `Ok(Some(value))`: the action was never interrupted by `Ctrl-C`
/// - `Ok(None)`: the action was interrupted by `Ctrl-C`
/// - `Err(e)`: either the inner task returned error, or some other error happened in the framework
///   (for example, when joining the task thread/future)
///
/// # Behavior
/// Since the calling thread is the one that polls the signal, the caller will be quickly
/// unblocked once the signal happens, and return `Ok(None)`. However, Rust does not
/// give us a way to "kill" the thread that is running the task. The task closure
/// receives an input `CtrlSignal` that it can use by periodically checking
/// if the task has been aborted
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// use std::thread;
/// use std::time::Duration;
///
/// match cu::cli::catch_ctrlc(|ctrlc| {
///     for _ in 0..10 {
///         cu::print!("please press Ctrl-C");
///         thread::sleep(Duration::from_secs(2));
///         ctrlc.check?; // returns Err if signaled
///     }
///     cu::Ok(42)
/// }) {
///     Ok(None) => cu::info!("was aborted!"),
///     Ok(Some(n)) => cu::info!("was finished: {n}"),
///     Err(e) => cu::error!("error: {e:?}"),
/// }
/// ```
///
/// Because we use polling and not a blocking join, `catch_ctrlc` can be used
/// even in async context. For the version that uses the async runtime, see [`co_catch_ctrlc`].
///
/// # Fallback
/// If the `Ctrl-C` framework failed to initialize, this may fallback to simply running
/// the task on the current thread, which will make it not abortable.
///
/// # Global Handlers
/// Action frames should be used whenever possible. However, if a global handler is 
/// needed,
/// Since the `ctrlc` crate only allows one global handler, and `cu` has registered it,
/// you need to use [`cu::cli::add_global_ctrlc_handler`] to register your global handlers.
///
/// Note that:
/// - Global handlers cannot be unregistered
///
///
/// # Action Frames
/// The `catch_ctrlc` function (this function) does the following:
/// - Push a new frame in the global Ctrl-C stack.
/// - Start executing the request in a new thread
/// - Block the current thread until either:
///   1. the task thread finishes and returns a result, or
///   2. the ctrl-c signal is received
///
/// The closure receives a signal object where it can use to check if 
/// termination is requested, so it can terminate the thread.
/// the thread will be joined before returning.
///
/// Since this blocks the current thread, use `co_catch_ctrlc` in async contexts
///
/// If it's signalled then this function is guaranteed to return Ok(None),
/// but the closure may or may not have executed.
/// if there are error registering ctrolc handler, we will execute
/// the closure without the ability to interrupt it
///
/// the closure is responsible for checking if it is interrupted and return
///
pub fn catch_ctrlc<T, F>(f: F) -> cu::Result<Option<T>> 
where 
    T: 'static + Send,
F: FnOnce(CtrlcSignal) -> cu::Result<T> + 'static + Send
{
    let signal = CtrlcSignal::new(crate::next_atomic_usize());
    {
        let Ok(mut signal_stack) = CTRLC_SIGNAL_STACK.lock() else {
            cu::trace!("failed to register new ctrl-c frame, will run synchronously");
            return f(signal).map(Some);
        };
        signal_stack.push(signal.clone());
    }

    let _drop_scope = CtrlcScope(signal.id);

    let handle = {
        let signal = signal.clone();
        std::thread::spawn(move || {
            // after the thread has spawned, check if it's already cancelled
            if signal.signaled() {
                return None;
            }
            Some(f(signal))
        })
    };
    if let Err(e) = &*INIT_ONCE {
        cu::bail!("{e}");
    }
    // poll for join
    while !handle.is_finished() {
        if signal.signaled() {
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    if signal.signaled() {
        return Ok(None);
    }
    match handle.join() {
        Err(e) => {
            cu::bail!("failed to join ctrl-c frame: {}", crate::best_effort_panic_info(&e));
        }
        Ok(None) => Ok(None) ,
        Ok(Some(v)) => Ok(Some(v?)),
    }

}

/// Async version of [`catch_ctrlc`].
///
/// For how the `Ctrl-C` framework works, please refer to [documentation](catch_ctrlc)
///
/// # Behavior
/// There are 2 main differences between this and the synchronous version:
/// 1. This does not spawn a dedicated thread. Instead, it's up to the async
///    runtime to execute the task.
/// 2. Even if the task does not check for the signal, the `Ctrl-C` signal
///    will cause the future to be dropped. In async terms, that means the future
///    will not be polled again if it yields back to the runtime (i.e. it
///    will be aborted)
///
/// In the example below, even we never check the signal, the warning
/// will not print if `Ctrl-C` was pressed in time.
/// ```rust,ignore
/// # use pistonite_cu as cu;
/// use std::time::Duration;
///
/// match cu::cli::co_catch_ctrlc(async |_| {
///     for _ in 0..10 {
///         cu::print!("please press Ctrl-C");
///         cu::co::sleep(Duration::from_secs(2)).await;
///     }
///     cu::warn!("did not abort!");
///     cu::Ok(42)
/// }).await {
///     Ok(None) => cu::info!("was aborted!"),
///     Ok(Some(n)) => cu::info!("was finished: {n}"),
///     Err(e) => cu::error!("error: {e:?}"),
/// }
/// ```
/// 
/// # Fallback
/// If the `Ctrl-C` framework failed to initialize, this may fallback to simply `.await`-ing
/// the task, which will make it not respond to `Ctrl-C` signals
///
#[cfg(feature = "coroutine")]
pub async fn co_catch_ctrlc<T, TFuture, F>(f: F) -> cu::Result<Option<T>> 
where 
    TFuture: Future<Output=cu::Result<T>> + Send + 'static,
F: FnOnce(CtrlcSignal) -> TFuture + Send + 'static,
    T: Send + 'static
{
    let signal = CtrlcSignal::new(crate::next_atomic_usize());
    {
        let Ok(mut signal_stack) = CTRLC_SIGNAL_STACK.lock() else {
            cu::trace!("failed to register new ctrl-c frame, will simply await");
            return f(signal).await.map(Some);
        };
        signal_stack.push(signal.clone());
    }

    let _drop_scope = CtrlcScope(signal.id);
    let handle = {
        let signal = signal.clone();
        cu::co::spawn(async move {
            // after the thread has spawned, check if it's already cancelled
            if signal.signaled() {
                return None;
            }
            Some(f(signal).await)
        })
    };
    if let Err(e) = &*INIT_ONCE {
        cu::bail!("{e}");
    }
    let sleep_future = 
            async {
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                if signal.signaled() {
                    break;
                }
            }
        };
    tokio::select! {
        result = handle.co_join() => {
            if signal.signaled() {
                return Ok(None);
            }
            match result {
                Err(e) => {
                    cu::bail!("failed to join ctrl-c frame: {e}");
                }
                Ok(None) => Ok(None),
                Ok(Some(v)) => Ok(Some(v?)),
            }
        }
        _ = sleep_future => {
            Ok(None)
        }
    }
}

#[derive(Clone)]
pub struct CtrlcSignal {
    id: usize,
    signaled_times: Arc<AtomicU8>
}
impl CtrlcSignal {
    fn new(id: usize) -> Self {
        Self { id, signaled_times: Arc::new(AtomicU8::new(0)) }
    }
    /// Return an `Err` if `Ctrl-C` has been signaled
    pub fn check(&self) -> cu::Result<()> {
        if self.signaled() {
            cu::bail!("interrupted")
        }
        Ok(())
    }
    /// Return `true` if `Ctrl-C` has been signaled
    pub fn signaled(&self) -> bool {
        self.signaled_times.load(Ordering::Acquire) > 0
    }
    /// Get the number of times `Ctrl-C` has been signaled
    pub fn signaled_times(&self) -> u8 {
        self.signaled_times.load(Ordering::Acquire)
    }
    /// Programmatically trigger the signal. Note that this does not
    /// send actual signal or keyboard events
    pub fn signal(&self) {
        self.signaled_times.fetch_add(1, Ordering::AcqRel);
    }
}
struct CtrlcScope(usize);
impl Drop for CtrlcScope {
    fn drop(&mut self) {
        if let Ok(mut signal_stack) = CTRLC_SIGNAL_STACK.lock() {
            signal_stack.retain(|x| x.id != self.0);
        }
    }
}
