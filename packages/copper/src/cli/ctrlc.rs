use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

/// New handlers that will be added by the ctrlc thread
/// once ctrlc is pressed
static CTRLC_HANDLERS_NEW: Mutex<Vec<Box<dyn FnMut() + Send>>> = Mutex::new(Vec::new());
/// Stack of CtrlC frames to signal
static CTRLC_SIGNAL_STACK: Mutex<Vec<CtrlcFrame>> = Mutex::new(Vec::new());
/// Thread safe init lock
static INIT_ONCE: LazyLock<Result<(), String>> = LazyLock::new(|| {
    let mut handlers = vec![];
    let set_result = ctrlc::try_set_handler(move || {
        // populate new handlers
        if let Ok(mut new_handlers) = CTRLC_HANDLERS_NEW.lock() {
            handlers.extend(new_handlers.drain(..));
        }
        // signal the stack
        let mut signalled = false;
        if let Ok(stack) = CTRLC_SIGNAL_STACK.lock() {
            if let Some(frame) = stack.last() {
                signalled = true;
                frame.signal.signal();
                if let Some(f) = &frame.on_signal {
                    f(frame.signal.clone())
                }
            }
        }

        // note we are not holding any lock when invoking user-defined handlers

        // if user did not set any global handler or action frames, then we terminate
        if !signalled && handlers.is_empty() {
            std::process::exit(1);
        }
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
/// See [Handling Ctrl-C](fn@crate::cli::ctrlc_frame).
///
/// Note that this is only available with feature `cli` - since
/// you should not be adding a global handler from a library.
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
/// way to set up a handler for `Ctrl-C`. `cu` builds wrappers around it so it can
/// be intergrated with other internals, such as prompts.
///
/// # Action Frames
/// Most of the time, custom Ctrl-C behavior is for executing some long running
/// tasks and give the user the ability to abort it.
/// The [`execute`](CtrlcBuilder::execute) function pushes a new frame
/// to an internal stack, then executes the task synchronously on the same thread.
/// **The task itself is responsible for periodically checking the received
/// signal object if it has been aborted (by calling `ctrlc.check?`).
///
/// The frame will only be removed from the stack when the task returns.
/// Note that different threads can spawn Ctrl-C frames, and those frame
/// may not finish in order. When user hits `Ctrl-C`, only the most recently
/// added frame (the top-most of the stack) will be notified.
/// It will continue to notify the frame until the task ends and removes
/// the frame from the stask.
///
/// # Async Behavior
/// The async version of the API, [`co_execute`](CtrlcBuilder::co_execute),
/// is exactly the same, other than it takes an async closure instead.
/// Even though in the async case, we can let the runtime drop the future
/// to abort it without explicit checks, the explicit checks make it clearer
/// and easier to reason about program states when aborting.
///
/// If you prefer automatically cancellation, consider this pattern:
#[cfg_attr(not(feature = "coroutine"), doc = "```rust,ignore")]
#[cfg_attr(feature = "coroutine", doc = "```rust,no_run")]
/// # use pistonite_cu as cu;
/// # async fn main_() -> cu::Result<()> {
/// let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
/// let result = cu::cli::ctrlc_frame()
///     .on_signal(move |_| { let _ = send.send(()); })
///     .co_execute(async move |ctrlc| {
///         let waiter = async move {
///             loop {
///                 if recv.recv().await.is_none() {
///                     return;
///                 }
///                 if ctrlc.should_abort() {
///                     return;
///                 }
///             }
///         };
///         tokio::select! {
///             result = my_long_running_task() => {
///                 return result;
///             }
///             _ = waiter => {
///                 // does not matter what the value is -
///                 // co_execute will ensure None is returned
///                 // when aborted
///                 return Ok(0);
///             }
///         }
///     }).await?;
/// match result {
///     Some(x) => cu::info!("valud is: {x}"),
///     None => cu::error!("aborted!"),
/// }
/// # Ok(()) }
///
/// async fn my_long_running_task() -> cu::Result<i32> {
///     // your task here can get aborted without checking the signal
///     // which can be dangerous and have unintended effects!
///     Ok(42)
/// }
/// ```
///
/// # Return Value
/// The return value of the inner `cu::Result<T>` will be transformed into
/// `cu::Result<Option<T>>`, where:
/// - `Ok(Some(value))`: the action was not aborted, and produced a value.
/// - `Ok(None)`: the action was aborted.
///   - The definition of "aborted" can be customized by setting the [`abort_threshold()`](CtrlcBuilder::abort_threshold)
///     on the builder
/// - `Err(e)`: either the inner task returned error, or some other error happened in the framework
///   (for example, when joining the task thread/future)
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// use std::thread;
/// use std::time::Duration;
///
/// match cu::cli::ctrlc_frame().execute(|ctrlc| {
///     for _ in 0..10 {
///         cu::print!("please press Ctrl-C");
///         thread::sleep(Duration::from_secs(2));
///         ctrlc.check()?; // returns Err if signaled
///     }
///     cu::Ok(42)
/// }) {
///     Ok(None) => cu::info!("was aborted!"),
///     Ok(Some(n)) => cu::info!("was finished: {n}"),
///     Err(e) => cu::error!("error: {e:?}"),
/// }
/// ```
///
/// # Fallback
/// If the `Ctrl-C` framework failed to initialize, this may fallback to simply running
/// the task without the ability to receive signals.
///
/// # Global Handlers
/// Action frames should be used whenever possible.
/// If you do need a global handler for `Ctrl-C`, use
/// [`cu::cli::add_global_ctrlc_handler`]. This is because the `ctrlc` crate
/// only allows one global handler.
///
/// Global handlers are called after the action frame is notified (if any),
/// and called in reverse order of registration.
/// The underlying handler is lazily set up whenever a global handler
/// or action frame is added. If there are no longer any action frames
/// and there are no global handlers, the underlying handler
/// will call `std::process::exit(1)` to terminate.
#[inline(always)]
pub fn ctrlc_frame() -> CtrlcBuilder {
    CtrlcBuilder::default()
}

/// Builder for a new frame for handling `Ctrl-C` signals.
/// The canonical factory function for this is `cu::cli::ctrlc_frame`.
/// See [Handling Ctrl-C](ctrlc_frame).
pub struct CtrlcBuilder {
    abort_threshold: u8,
    on_signal: Option<OnSignalFn>,
}
impl Default for CtrlcBuilder {
    #[inline(always)]
    fn default() -> Self {
        Self {
            abort_threshold: 1,
            on_signal: None,
        }
    }
}
impl CtrlcBuilder {
    /// Set the number of `Ctrl-C` signals required for the task
    /// to be considered aborted (where the return will be `Ok(None`).
    ///
    /// Default is 1
    #[inline(always)]
    pub fn abort_threshold(mut self, threshold: u8) -> Self {
        self.abort_threshold = threshold;
        self
    }

    /// Set a function to be called when `Ctrl-C` signal is received.
    ///
    /// The function will be executed on the `Ctrl-C` signal handling thread,
    /// not the thread that runs the task, and is
    #[inline(always)]
    pub fn on_signal<F: Fn(CtrlcSignal) + Send + 'static>(mut self, f: F) -> Self {
        self.on_signal = Some(Box::new(f));
        self
    }

    /// Execute the task
    pub fn execute<T, F>(self, f: F) -> cu::Result<Option<T>>
    where
        F: FnOnce(CtrlcSignal) -> cu::Result<T>,
    {
        let signal = CtrlcSignal::new(self.abort_threshold);
        let Some(ctrlc_frame_scope) = CtrlcFrame::push_scope(signal.clone(), self.on_signal) else {
            return f(signal).map(Some);
        };
        if let Err(e) = &*INIT_ONCE {
            cu::bail!("{e}");
        }
        let result = f(signal.clone());
        if signal.should_abort() {
            return Ok(None);
        }
        drop(ctrlc_frame_scope); // suppress unused warning
        result.map(Some)
    }

    #[cfg(feature = "coroutine")]
    pub async fn co_execute<T, TFuture, F>(self, f: F) -> cu::Result<Option<T>>
    where
        TFuture: Future<Output = cu::Result<T>>,
        F: FnOnce(CtrlcSignal) -> TFuture,
    {
        let signal = CtrlcSignal::new(self.abort_threshold);
        let Some(ctrlc_frame_scope) = CtrlcFrame::push_scope(signal.clone(), self.on_signal) else {
            return f(signal).await.map(Some);
        };
        if let Err(e) = &*INIT_ONCE {
            cu::bail!("{e}");
        }
        let result = f(signal.clone()).await;
        if signal.should_abort() {
            return Ok(None);
        }
        drop(ctrlc_frame_scope); // suppress unused warning
        result.map(Some)
    }
}

type OnSignalFn = Box<dyn Fn(CtrlcSignal) + Send>;
struct CtrlcFrame {
    id: usize,
    signal: CtrlcSignal,
    on_signal: Option<OnSignalFn>,
}
struct CtrlcScope(usize);
/// Signal passed into a task executing inside a `Ctrl-C` action frame,
/// for it to check if `Ctrl-C` has been signaled
#[derive(Clone)]
pub struct CtrlcSignal {
    signaled_times: Arc<AtomicU8>,
    abort_threshold: u8,
}
impl CtrlcFrame {
    pub fn push_scope(signal: CtrlcSignal, on_signal: Option<OnSignalFn>) -> Option<CtrlcScope> {
        let Ok(mut signal_stack) = CTRLC_SIGNAL_STACK.lock() else {
            cu::trace!("failed to register new ctrl-c frame");
            return None;
        };
        let id = crate::next_atomic_usize();
        signal_stack.push(Self {
            id,
            signal,
            on_signal,
        });
        Some(CtrlcScope(id))
    }
}
impl Drop for CtrlcScope {
    fn drop(&mut self) {
        if let Ok(mut signal_stack) = CTRLC_SIGNAL_STACK.lock() {
            signal_stack.retain(|x| x.id != self.0);
        }
    }
}

impl CtrlcSignal {
    fn new(abort_threshold: u8) -> Self {
        Self {
            signaled_times: Arc::new(AtomicU8::new(0)),
            abort_threshold,
        }
    }
    /// Return an `Err` if `Ctrl-C` has been signaled
    pub fn check(&self) -> cu::Result<()> {
        if self.should_abort() {
            cu::bail!("interrupted")
        }
        Ok(())
    }
    /// Return `true` if `Ctrl-C` has been signaled at least
    /// the same number of times as the abort_threshold
    pub fn should_abort(&self) -> bool {
        self.signaled_times() >= self.abort_threshold
    }
    /// Return `true` if `Ctrl-C` has been signaled at least once
    pub fn signaled(&self) -> bool {
        self.signaled_times() > 0
    }
    /// Get the number of times `Ctrl-C` has been signaled
    pub fn signaled_times(&self) -> u8 {
        self.signaled_times.load(Ordering::Acquire)
    }
    /// Programmatically trigger the signal. Note that this does not
    /// send actual signal or keyboard events
    pub fn signal(&self) {
        self.signaled_times.fetch_add(1, Ordering::SeqCst);
    }
}
