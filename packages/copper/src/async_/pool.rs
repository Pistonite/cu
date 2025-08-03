use std::sync::Arc;

use tokio::sync::Semaphore;

use super::Handle;

/// A pool for limiting async tasks
///
/// The pool is not to be used as a thread pool. It is used more as a "limiter"
/// to ensure we only fire a limited number of resource-intensive tasks.
/// The IO work for those tasks are drived by a single monitoring thread.
///
/// This is good for things like spawning compiler processes where the IO between
/// child and parent processes are low, but not good for spawning 
/// CPU-bound tasks. Use something like `rayon` for CPU parallelism.
///
/// The pool can be cloned and shared between threads. Dropping the pool will not cause the spawned tasks to be either joined
/// to canceled. You must use individual handles to join them.
#[derive(Clone)]
pub struct Pool(Arc<PoolInner>);
struct PoolInner(Semaphore);
impl Pool {
    /// Create a new pool with capacity, panics if the capacity is 0
    pub fn new(capacity: usize) -> Self {
        if capacity == 0 {
            crate::panicand!(error!("cannot create a pool with 0 capacity"));
        }
        Self(Arc::new(PoolInner(Semaphore::new(capacity))))
    }

    /// Spawn a task using the background runtime
    ///
    /// The task is spawned with [`cu::co::spawn`](crate::co::spawn),
    /// and will only start being executed when the pool
    /// has availability (permits).
    ///
    /// If you are in an async context and want to spawn the task
    /// onto the current runtime context, use [`co_spawn`](Self::co_spawn)
    pub fn spawn<F>(&self, future: F) -> Handle<F::Output>
where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let sem = Arc::clone(&self.0);
        crate::co::spawn(Self::wrapped_future(sem, future))
    }

    /// Spawn a task using the active runtime
    ///
    /// The task is spawned with [`cu::co::co_spawn`](crate::co::co_spawn),
    /// and will only start being executed when the pool
    /// has availability (permits).
    ///
    /// Will panic if not inside a runtime context.
    pub fn co_spawn<F>(&self, future: F) -> Handle<F::Output>
where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let sem = Arc::clone(&self.0);
        crate::co::co_spawn(Self::wrapped_future(sem, future))
    }

    fn wrapped_future<F>(sem: Arc<PoolInner>, future: F) -> impl Future<Output=F::Output> 
where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        async move {
            let _permit = sem.0.acquire().await.ok();
            let result  =future.await;
            drop(_permit);
            result
        }
    }
}
