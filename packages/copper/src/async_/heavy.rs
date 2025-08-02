use std::sync::LazyLock;

use tokio::runtime::{Builder, Runtime};


/// the heavy tokio runtime for async tasks with CPU-bound work
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread().enable_all().build()
    .expect("cannot create heavy tokio runtime")
});

/// Run a heavy async job and block the current thread until
/// it's done. 
///
/// If the async work is not CPU-bound, and other CPU-bound work
/// is happening, consider using [`cu::run`](crate::run) or [`cu::spawn`](crate::spawn)
/// to run it only with the current thread or the worker thread, respectively.
pub fn run_heavy<F>(future: F) -> F::Output
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    RUNTIME.block_on(future)
}
