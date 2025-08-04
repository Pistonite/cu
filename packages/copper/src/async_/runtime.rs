use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, LazyLock};

use tokio::runtime::{Builder, Runtime};
use tokio::task::{JoinHandle, JoinError};

/// the current-thread runtime
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_current_thread().enable_all().build()
    .expect("cannot create current-thread tokio runtime")
});

/// the multi-threaded, background runtime
static BACKGROUND_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    #[cfg(feature="coroutine-heavy")]
    {
        Builder::new_multi_thread().enable_all().build()
            .expect("cannot create heavy background tokio runtime")
    }
    #[cfg(not(feature="coroutine-heavy"))]
    {
        Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all().build()
            .expect("cannot create background tokio runtime")
    }
});

/// Spawn a task onto the background runtime
#[inline]
pub fn spawn<F>(future: F) -> Handle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static
{
    Handle(BACKGROUND_RUNTIME.spawn(future))
}

/// Spawn a task onto the current runtime context. Will panic if not 
/// inside a runtime context
#[inline]
pub fn co_spawn<F>(future: F) -> Handle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static
{
    Handle(tokio::spawn(future))
}

/// Run an async task using the current thread.
///
/// If the task is heavy, consider using [`spawn`] to run
/// it on a background thread
#[inline]
pub fn run<F>(future: F) -> F::Output
where
    F: Future + Send + 'static,
    F::Output: Send + 'static {

    RUNTIME.block_on(future)
}

/// Run an async task using the background runtime
#[inline]
pub fn run_bg<F>(future: F) -> F::Output
where
    F: Future + Send + 'static,
    F::Output: Send + 'static {

    BACKGROUND_RUNTIME.block_on(future)
}

/// Join handle for async task
///
/// This is a wrapper around `tokio`'s `JoinHandle` type.
pub struct Handle<T>(JoinHandle<T>);
impl<T> Handle<T> {
    /// Convert this handle into a [`RobustHandle`].
    /// with a more robust mechanism for aborting.
    pub fn into_robust(self) -> RobustHandle<T> {
        self.into()
    }

    /// Abort the task, trying to `join` or `co_join` an aborted
    /// task (if it's not already completed) will return an error indicating
    /// it's already aborted.
    ///
    /// If the task was completed before joining, however, it may not
    /// see this abort call. If you always want `join` to indicate
    /// aborted, see [`RobustHandle`]
    pub fn abort(&self) {
        self.0.abort();
    }

    /// Return a handle to remotely abort the task
    pub fn abort_handle(&self) -> AbortHandle {
        self.0.abort_handle()
    }

    /// Block the current thread to join the task
    ///
    /// Panics are caught by the runtime, and will be returned as an Err.
    ///
    /// Will error if the task was aborted or panicked.
    /// If you want to handle the abort, use [`join_maybe_aborted`](Self::join_maybe_aborted)
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will potentially
    /// block the runtime. Use [`co_join().await`](`Self::co_join`) instead.
    #[inline]
    pub fn join(self) -> crate::Result<T> {
        Self::handle_error(RUNTIME.block_on(self.0))
    }

    /// Wait for the task asynchronously
    ///
    /// Panics are caught by the runtime, and will be returned as an Err.
    ///
    /// Will error if the task was aborted or panicked.
    /// If you want to handle the abort, use [`co_join_maybe_aborted`](Self::join_maybe_aborted)
    #[inline]
    pub async fn co_join(self) -> crate::Result<T> {
        Self::handle_error(self.0.await)
    }

    /// Like [`join`](Self::join), but returns `None` if the task was aborted.
    ///
    /// Panics are caught by the runtime, and will be returned as an Err.
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will potentially
    /// block the runtime. Use [`co_join_maybe_aborted().await`](`Self::co_join_maybe_aborted`) instead.
    #[inline]
    pub fn join_maybe_aborted(self) -> crate::Result<Option<T>> {
        Self::handle_error_maybe_aborted(RUNTIME.block_on(self.0))
    }

    /// Like [`co_join`](Self::co_join), but returns `None` if the task was aborted.
    #[inline]
    pub async fn co_join_maybe_aborted(self) -> crate::Result<Option<T>> {
        Self::handle_error_maybe_aborted(self.0.await)
    }

    #[inline]
    fn handle_error(e: Result<T, JoinError>) -> crate::Result<T> {
        match Self::handle_error_maybe_aborted(e) {
            Ok(Some(x)) => Ok(x),
            Ok(None) => crate::bail!("aborted"),
            Err(e) => Err(e)
        }
    }

    fn handle_error_maybe_aborted(e: Result<T, JoinError>) -> crate::Result<Option<T>> {
        let e = match e {
            Ok(x) => return Ok(Some(x)),
            Err(e) => e
        };
        Self::handle_join_error(e)?;
        Ok(None)
    }

    // return Ok if the error is abort
    fn handle_join_error(e: JoinError) -> crate::Result<()> {
        let e = match e.try_into_panic() {
            Ok(panic) => {
                let info = crate::best_effort_panic_info(&panic);
                crate::bail!("task panicked: {info}");
            }
            Err(e) => e
        };
        if e.is_cancelled() {
            Ok(())
        } else {
            crate::bail!("failed to join task due to unknown reason: {e:?}")
        }
    }
}

pub type AbortHandle = tokio::task::AbortHandle;

/// Join handle for async task, like [`Handle`], with a more robust mechanism for aborting.
///
/// This is more robust than [`Handle::abort`], that aborting
/// a completed task is possible as long as it's not joined yet
/// (even if `join` has already been called)
pub struct RobustHandle<T> {
    inner: Handle<T>,
    aborted: Arc<AtomicU8>
}
impl<T> From<Handle<T>> for RobustHandle<T> {
    fn from(value: Handle<T>) -> Self {
        Self {
            inner: value,
            aborted: Arc::new(AtomicU8::new(0))
        }
    }
}

impl<T> RobustHandle<T> {
    /// Abort the [`RobustHandle`] pointed to by this handle.
    ///
    /// Returns if the abort was successful. If `false` is returned,
    /// that means the task has already been joined.
    ///
    /// See documentation [above](RobustHandle) for more detail
    pub fn abort(&self) -> bool {
        let ok = RobustAbortHandle::abort_internal(&self.aborted);
        if ok {
            self.inner.abort();
        }
        ok
    }

    /// Return a handle to remotely and robustly abort the task
    pub fn abort_handle(&self) -> RobustAbortHandle {
        RobustAbortHandle { inner: self.inner.abort_handle(), aborted: Arc::clone(&self.aborted) }
    }

    /// Block the current thread to join the task
    ///
    /// Panics are caught by the runtime, and will be returned as an Err.
    ///
    /// Will error if the task was aborted or panicked.
    /// If you want to handle the abort, use [`join_maybe_aborted`](Self::join_maybe_aborted)
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will potentially
    /// block the runtime. Use [`co_join().await`](`Self::co_join`) instead.
    #[inline]
    pub fn join(self) -> crate::Result<T> {
        match self.join_maybe_aborted()? {
            Some(x) => Ok(x),
            None => crate::bail!("aborted"),
        }
    }

    /// Wait for the task asynchronously
    ///
    /// Panics are caught by the runtime, and will be returned as an Err.
    ///
    /// Will error if the task was aborted or panicked.
    /// If you want to handle the abort, use [`co_join_maybe_aborted`](Self::join_maybe_aborted)
    #[inline]
    pub async fn co_join(self) -> crate::Result<T> {
        match self.co_join_maybe_aborted().await? {
            Some(x) => Ok(x),
            None => crate::bail!("aborted"),
        }
    }

    /// Like [`join`](Self::join), but returns `None` if the task was aborted.
    ///
    /// Panics are caught by the runtime, and will be returned as an Err.
    ///
    /// The task may still be completed with a value if it was aborted.
    /// If you want to access it, see [`join_maybe_aborted_robust`](Self::join_maybe_aborted_robust)
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will potentially
    /// block the runtime. Use [`co_join_maybe_aborted().await`](`Self::co_join_maybe_aborted`) instead.
    #[inline]
    pub fn join_maybe_aborted(self) -> crate::Result<Option<T>> {
        match self.join_maybe_aborted_robust()? {
            Ok(x) => Ok(Some(x)),
            Err(_) => Ok(None)
        }
    }

    /// Like [`join`](Self::join), but returns `None` if the task was aborted.
    ///
    /// Panics are caught by the runtime, and will be returned as an Err.
    ///
    /// The task may still be completed with a value if it was aborted.
    /// If you want to access it, see [`co_join_maybe_aborted_robust`](Self::co_join_maybe_aborted_robust)
    #[inline]
    pub async fn co_join_maybe_aborted(self) -> crate::Result<Option<T>> {
        match self.co_join_maybe_aborted_robust().await? {
            Ok(x) => Ok(Some(x)),
            Err(_) => Ok(None)
        }
    }

    /// Block the current thread to join the task
    ///
    /// Panics are caught by the runtime, and will be returned as an Err.
    ///
    /// # Return Value
    /// The outer `Result` checks if joining is successful, and the inner
    /// indicates if the task was aborted. If the task was completed before
    /// it was aborted, the value is also returned.
    ///
    /// - Returns `Ok(Ok(T))` if join is successful and task is not aborted
    /// - Returns `Ok(Err(None))` if join is successful, and task is aborted
    ///   without finishing
    /// - Returns `Ok(Err(Some(T)))` if join is successful, and task is aborted,
    ///   but it was finished anyway.
    /// - Returns `Err` if join fails.
    ///
    /// ```rust
    /// use std::time::Duration;
    /// 
    /// let handle = cu::co::spawn(async move {
    ///     tokio::time::sleep(Duration::from_millis(10)).await;
    ///     42
    /// }).into_robust();
    ///
    /// std::time::sleep(Duration::from_millis(20));
    /// assert!(handle.abort(), "task is not joined yet, so abort is possible");
    /// match handle.join_maybe_aborted_robust() {
    ///     Err(e) => panic!("join failed: {e}"),
    ///     Ok(Ok(x)) => {
    ///         assert!(false, "abort was called, so it can't return Ok(Ok)")
    ///     }
    ///     Ok(Err(Some(x))) => {
    ///         assert_eq!(x, 42, "abort was called after task has already produced value 42");
    ///     }
    ///     Ok(Err(None)) => {
    ///         assert!(false, "abort was not called when the task was still running ")
    ///     }
    /// }
    /// ```
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will potentially
    /// block the runtime. Use [`co_join_maybe_aborted_robust().await`](`Self::co_join_maybe_aborted_robust`) instead.
    pub fn join_maybe_aborted_robust(self) -> crate::Result<Result<T, Option<T>>> {
        Self::handle_error_maybe_aborted_robust(RUNTIME.block_on(self.inner.0), &self.aborted)
    }

    /// Wait for the task asynchronously
    ///
    /// Panics are caught by the runtime, and will be returned as an Err.
    ///
    /// # Return Value
    /// See [`join_maybe_aborted_robust`](Self::join_maybe_aborted_robust)
    pub async fn co_join_maybe_aborted_robust(self) -> crate::Result<Result<T, Option<T>>> {
        Self::handle_error_maybe_aborted_robust(self.inner.0.await, &self.aborted)
    }

    fn handle_error_maybe_aborted_robust(e: Result<T, JoinError>, aborted: &AtomicU8) -> crate::Result<Result<T, Option<T>>> {
        let e = match e {
            Ok(x) => {
                if Self::check_aborted(aborted) {
                    return Ok(Err(Some(x)));
                } else {
                    return Ok(Ok(x));
                }
            }
            Err(e) => e
        };
        if Self::check_aborted(aborted) {
            return Ok(Err(None));
        }
        Handle::<T>::handle_join_error(e)?;
        Ok(Err(None))
    }

    fn check_aborted(aborted: &AtomicU8) -> bool {
        loop {
            let status = aborted.load(Ordering::Relaxed);
            // aborted 
            if status == 1 {
                return true;
            }
            debug_assert_eq!(0, status, "only the join handle can set the status to 2");
            if aborted.compare_exchange_weak(0, 2, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                return false;
            }
            std::hint::spin_loop();
        }
    }
}

pub struct RobustAbortHandle {
    inner: AbortHandle,
    aborted: Arc<AtomicU8>
}
impl RobustAbortHandle {
    /// Abort the [`RobustHandle`] pointed to by this handle.
    ///
    /// See documentation for [`RobustHandle`] for more details.
    ///
    /// Returns if the abort was successful. If `false` is returned,
    /// that means the task has already been joined.
    pub fn abort(&self) -> bool {
        let ok = Self::abort_internal(&self.aborted);
        if ok {
            self.inner.abort();
        }
        ok
    }

    fn abort_internal(aborted: &AtomicU8) -> bool {
        loop {
            let status = aborted.load(Ordering::Relaxed);
            // already joined
            if status == 2 {
                return false;
            }
            // aborted
            if status != 0 {
                debug_assert_eq!(status, 1);
                return true;
            }
            if aborted.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                return true;
            }
            std::hint::spin_loop();
        }
    }
}
