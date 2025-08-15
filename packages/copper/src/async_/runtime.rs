use std::sync::LazyLock;

use tokio::runtime::{Builder, Runtime};

use super::Handle;

/// the current-thread runtime
#[cfg(not(feature = "coroutine-heavy"))]
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("cannot create current-thread tokio runtime")
});

/// the multi-threaded, background runtime
static BACKGROUND_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    #[cfg(feature = "coroutine-heavy")]
    {
        Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("cannot create heavy background tokio runtime")
    }
    #[cfg(not(feature = "coroutine-heavy"))]
    {
        Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("cannot create background tokio runtime")
    }
});

/// Get a reference of a runtime that contains the current thread
pub(crate) fn foreground() -> &'static Runtime {
    #[cfg(not(feature = "coroutine-heavy"))]
    {
        &RUNTIME
    }
    #[cfg(feature = "coroutine-heavy")]
    {
        &BACKGROUND_RUNTIME
    }
}

pub(crate) fn background() -> &'static Runtime {
    &BACKGROUND_RUNTIME
}

/// Run an async task using the current thread.
///
/// To prevent misuse, this is only available without the `coroutine-heavy`
/// feature. Consider this entry point to some async procedure, if most of
/// your program is sync.
///
/// Use [`spawn`] or [`run`] to run async tasks using the background thread(s)
/// in both light and heavy async use cases.
#[inline]
#[cfg(not(feature = "coroutine-heavy"))]
pub fn block<F>(future: F) -> F::Output
where
    F: Future,
{
    RUNTIME.block_on(future)
}

/// Spawn a task onto the background runtime
#[inline]
pub fn spawn<F>(future: F) -> Handle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    Handle(BACKGROUND_RUNTIME.spawn(future))
}

/// Spawn a task onto the blocking pool of the background runtime
///
/// Since the light context only has one background thread,
/// this is only enabled in heavy context to prevent misuse.
#[inline]
#[cfg(feature = "coroutine-heavy")]
pub fn spawn_blocking<F, R>(func: F) -> Handle<F::Output>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    Handle(BACKGROUND_RUNTIME.spawn_blocking(func))
}

/// Run an async task using the background runtime
#[inline]
pub fn run<F>(future: F) -> F::Output
where
    F: Future,
{
    BACKGROUND_RUNTIME.block_on(future)
}
