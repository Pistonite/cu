use std::{sync::{Arc, LazyLock, Mutex, TryLockError, TryLockResult, atomic::{AtomicBool, AtomicU8, Ordering}}, thread::JoinHandle, time::Duration};


use spin::Mutex as SpinMutex;


static CTRLC_HANDLERS_NEW: Mutex<Vec<Box<dyn FnMut() +Send>>> = Mutex::new(Vec::new());
static CTRLC_SIGNAL_STACK: Mutex<Vec<CtrlcSignal>> = Mutex::new(Vec::new());
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

pub async fn co_catch_ctrlc<T, TFuture, F>(f: F) -> cu::Result<Option<T>> 
where 
    TFuture: Future<Output=cu::Result<T>>,
F: FnOnce(CtrlcSignal) -> TFuture,
    T: Send
{
    let signal = CtrlcSignal::new(crate::next_atomic_usize());
    {
        let Ok(mut signal_stack) = CTRLC_SIGNAL_STACK.lock() else {
            cu::trace!("failed to register new ctrl-c frame, will run synchronously");
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
    tokio::select! {
        result = handle.co_join_maybe_aborted().await => {
        }
    }
}

#[derive(Clone)]
struct CtrlcSignal {
    id: usize,
    signaled_times: Arc<AtomicU8>
}
impl CtrlcSignal {
    fn new(id: usize) -> Self {
        Self { id, signaled_times: Arc::new(AtomicU8::new(0)) }
    }
    pub fn check_signaled(&self) -> cu::Result<()> {
        if self.signaled() {
            cu::bail!("interrupted")
        }
        Ok(())
    }
    pub fn signaled(&self) -> bool {
        self.signaled_times.load(Ordering::Acquire) > 0
    }
    pub fn signaled_times(&self) -> u8 {
        self.signaled_times.load(Ordering::Acquire)
    }
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
