use std::sync::{Arc, Weak};

use tokio::sync::Semaphore;

use crate::co::{AbortHandle, Handle, co_util, runtime};

/// Create a new [`Pool`].
///
/// - If `capacity > 0`, then the pool will be created
///   with literally that number of permits (concurrency) available.
/// - If `capacity = 0`, then the pool will be created
///   with the number of logical processors on the system
///   using the `num_cpus` crate, minimum 1.
/// - if `capacity < 0`, then the pool will be created
///   with the number of logical processors on the system,
///   minus the specified amount, and minimum 1.
///   Use `pool(-1)`, for example, to fire off background async tasks
///   while the main thread is doing more work.
#[inline(always)]
pub fn pool(capacity: isize) -> Pool {
    match capacity {
        1.. => Pool::new(capacity as usize),
        c => {
            let n = num_cpus::get();
            let n = n.saturating_sub(-c as usize).max(1);
            Pool::new(n)
        }
    }
}

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
///
/// This is constructed with [`cu::co::pool`](function@pool),
/// since it's shorter than `cu::co::Pool::new(n)`.
#[derive(Clone)]
pub struct Pool(Arc<PoolInner>);
struct PoolInner(Semaphore);
impl Pool {
    fn new(capacity: usize) -> Self {
        // this new function is meant to be a plain constructor,
        // so the extra logic for user-friendly interface
        // is in the `pool` function.
        //
        // this function is also private, so there's only one way
        // for user to create a Pool.
        Self(Arc::new(PoolInner(Semaphore::new(capacity))))
    }

    /// Add N permits to the underlying semaphore
    pub fn add_permits(&self, n: usize) {
        self.0.0.add_permits(n);
    }

    /// Remove N permits from the underlying semaphore,
    /// return the actual number removed.
    pub fn forget_permits(&self, n: usize) -> usize {
        self.0.0.forget_permits(n)
    }

    /// Spawn a task using the background runtime
    ///
    /// The task is spawned with [`cu::co::spawn`](crate::co::spawn),
    /// and will only start being executed when the pool
    /// has availability (permits).
    pub fn spawn<F>(&self, future: F) -> Handle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let sem = Arc::clone(&self.0);
        crate::co::spawn(async move {
            let _permit = sem.0.acquire().await.ok();
            let result = future.await;
            drop(_permit);
            result
        })
    }

    /// Spawn a synchronous task using the background runtime
    ///
    /// A wait task will be spawned with [`cu::co::spawn`](crate::co::spawn),
    /// and the function will only be spawned with `cu::co::spawn_blocking`
    /// when the pool has availability (permits).
    ///
    /// # Performance
    /// Even though it's never recommended to block the async runtime,
    /// if the synchronous work is short, it might be more performant
    /// to use `spawn` instead.
    #[cfg(feature = "coroutine-heavy")]
    pub fn spawn_blocking<F, R>(&self, f: F) -> Handle<crate::Result<R>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let sem = Arc::clone(&self.0);
        crate::co::spawn(async move {
            let _permit = sem.0.acquire().await.ok();
            let result = crate::co::spawn_blocking(f).co_join().await;
            drop(_permit);
            result
        })
    }
}

/// Create a set of handles, that can be joined concurrently.
///
/// Typically, you would spawn the tasks and collect the handles in a `Vec`,
/// and convert it to a set with this function. Later, you can use [`Set::extend`]
/// to add more handles.
pub fn set<T: Send + 'static, I: IntoIterator<Item = Handle<T>>>(iter: I) -> Set<T> {
    let mut set = Set::new();
    set.extend(iter);
    set
}

/// Create a set of handles, that can be joined concurrently.
///
/// The input handles are allowed to contain errors. Those errors
/// will be collected and returned along with the set in an `Err`.
/// If there are no errors, `Ok` is returned.
///
/// Typically, you would spawn the tasks and collect the handles in a `Vec`,
/// and convert it to a set with this function. Later, you can use [`Set::extend`]
/// to add more handles.
pub fn set_flatten<T: Send + 'static, E, I: IntoIterator<Item = Result<Handle<T>, E>>>(
    iter: I,
) -> Result<Set<T>, (Set<T>, Vec<E>)> {
    let mut set = Set::new();
    let errors = set.extend_flatten(iter);
    if errors.is_empty() {
        Ok(set)
    } else {
        Err((set, errors))
    }
}

/// A set of [`Handle`]s that can be joined concurrently.
///
/// When the set is dropped, all handles added to the set
/// are aborted.
pub struct Set<T> {
    join_set: tokio::task::JoinSet<crate::Result<T>>,
    abort_handles: Vec<Weak<AbortHandle>>,
}
impl<T> Drop for Set<T> {
    fn drop(&mut self) {
        for x in &self.abort_handles {
            if let Some(x) = x.upgrade() {
                x.abort()
            }
        }
    }
}
impl<T: Send + 'static> Set<T> {
    fn new() -> Self {
        Self {
            join_set: tokio::task::JoinSet::new(),
            abort_handles: Vec::new(),
        }
    }

    /// Add one handle to the set.
    ///
    /// A task will be spawned onto the background runtime to start joining
    /// the handle. Therefore, when there are a lot of handles,
    /// it's better to collect them first into a Vec, then use [`extend`](Self::extend)
    /// to add them to the set at once, to put slightly less pressure on the runtime.
    ///
    /// The handle will also become not abortable, but dropping the Set will
    /// abort all handles.
    pub fn add(&mut self, handle: Handle<T>) {
        self.gc_handles(Some(1));
        self.add_internal(handle);
    }

    /// Add multiple handles to the set
    ///
    /// The handles will become not abortable, but dropping the Set will
    /// abort all handles.
    pub fn extend<I: IntoIterator<Item = Handle<T>>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.gc_handles(iter.size_hint().1);
        for handle in iter {
            self.add_internal(handle);
        }
    }

    /// Add multiple handles to the set, from a set of results, the errors are collected
    /// and returned.
    ///
    /// The handles will become not abortable, but dropping the Set will
    /// abort all handles.
    pub fn extend_flatten<E, I: IntoIterator<Item = Result<Handle<T>, E>>>(
        &mut self,
        iter: I,
    ) -> Vec<E> {
        let iter = iter.into_iter();
        let mut errors = vec![];
        self.gc_handles(iter.size_hint().1);
        for handle in iter {
            match handle {
                Ok(handle) => self.add_internal(handle),
                Err(e) => errors.push(e),
            }
        }
        errors
    }

    fn add_internal(&mut self, handle: Handle<T>) {
        let abort_handle = Arc::new(handle.abort_handle());
        self.abort_handles.push(Arc::downgrade(&abort_handle));
        self.join_set.spawn_on(
            async move {
                let result = handle.co_join().await;
                // allow the abort handle to be cleaned up later
                drop(abort_handle);
                result
            },
            runtime::background().handle(),
        );
    }

    fn gc_handles(&mut self, add: Option<usize>) {
        // gc whenever we are about to grow
        if let Some(add) = add
            && self.abort_handles.capacity() - self.abort_handles.len() >= add
        {
            return;
        }
        self.abort_handles.retain(|x| x.strong_count() != 0);
    }

    /// Wait for the next handle to be available, and return it's result,
    /// or `None` if the set is empty, meaning all handles are joined.
    pub async fn next(&mut self) -> Option<crate::Result<T>> {
        let result = self.join_set.join_next().await?;
        match result {
            Err(join_error) => match co_util::handle_join_error(join_error) {
                Err(e) => Some(Err(e)),
                Ok(_) => Some(Err(crate::fmterr!("aborted"))),
            },
            Ok(x) => Some(x),
        }
    }

    /// Wait for the next handle to be available, and return it's result,
    /// or `None` if the set is empty, meaning all handles are joined.
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will block the runtime,
    /// and will panic if the thread is currently driving IO.
    /// Use [`next().await`](`Self::next`) instead.
    #[inline]
    pub fn block(&mut self) -> Option<crate::Result<T>> {
        runtime::foreground().block_on(async move { self.next().await })
    }
}
