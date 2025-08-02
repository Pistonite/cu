use std::sync::Arc;

use tokio::sync::Semaphore;

use super::{JoinHandle, LwHandle};


/// A pool for limiting async tasks
///
/// The pool is not to be used as a thread pool. It is used more as a "limiter"
/// to ensure we only fire a limited number of resource-intensive tasks.
/// The IO work for those tasks are drived by a single monitoring thread.
///
/// This is good for things like spawning compiler processes where the IO between
/// child and parent processes are low, but not good for spawning 
/// CPU-bound tasks. Use `rayon`, [`cu::run`](crate::run), or [`cu::run_heavy`](crate::run_heavy)
/// for CPU-bound or blocking tasks.
///
/// All jobs spawned with the pool are spawned on the worker thread,
/// same as [`cu::spawn`](crate::spawn). Note that only tasks spawned
/// with the pool are limited, and not other pools or `spawn` calls.
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

    /// Spawn a task, which will only start being executed when the pool
    /// has availability (permits)
    pub fn spawn<F>(&self, future: F) -> LwHandle<F::Output>
where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let sem = Arc::clone(&self.0);
        crate::co::spawn(async move {
            let _permit = sem.0.acquire().await.ok();
            let result  =future.await;
            drop(_permit);
            result
        })
    }

    pub fn co_spawn<F>(&self, future: F) -> JoinHandle<F::Output>
where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let sem = Arc::clone(&self.0);
        tokio::spawn(async move {
            let _permit = sem.0.acquire().await.ok();
            let result  =future.await;
            drop(_permit);
            result
        })
    }
}
