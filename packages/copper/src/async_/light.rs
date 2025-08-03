use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::{Arc, LazyLock, atomic::AtomicUsize};
use std::thread::ThreadId;
use std::time::Duration;

use tokio::runtime::{Builder, Runtime};

use super::BoxedFuture;

struct LwThread {
    spawn: Sender<BoxedFuture<()>>,
    id: ThreadId
}

/// Handle for an async task running on the light-weight async thread
pub struct LwHandle<T> {
    recv: oneshot::Receiver<T>,
}

impl<T> LwHandle<T> {
    /// Block the current thread on joining the handle from light-weight thread.
    /// Calling this from the light weight thread will panic.
    pub fn join(self) -> crate::Result<T> {
        if std::thread::current().id() == THREAD.id {
            panic!("cannot join light-weight handle from light-weight thread itself, consider using a co_* function to spawn the task instead.");
        }

        use crate::Context as _;
        self.recv.recv().context("failed to join an async handle")
    }

    pub fn try_join(&self) -> crate::Result<Option<T>> {
        // no need to check thread here because this is non-blocking
        use crate::Context as _;
        match self.recv.try_recv() {
            Ok(x) => Ok(Some(x)),
            Err(oneshot::TryRecvError::Empty) => Ok(None),
            Err(e) => Err(e).context("failed to join an async handle")
        }
    }
}

/// A light weight worker thread that handles async tasks
/// using a single-threaded tokio runtime
static THREAD: LazyLock<LwThread> = LazyLock::new(|| {
    // we only need one thread since the tasks are lightweight
    let (send, recv) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime for async light weight tasks");
        // runtime loop
        runtime.block_on(async move {
            let active_count = Arc::new(AtomicUsize::new(0));
            let mut recv = recv;
            loop {
                crate::trace!("blocking on lw thread");
                // Block the entire runtime, until we get a task
                let Ok(fut) = recv.recv() else {
                    break;
                };
                crate::trace!("inc base");
                active_count.fetch_add(1, Ordering::SeqCst);
                let base = {
                    let active_count = Arc::clone(&active_count);
                    tokio::spawn(async move { 
                        fut.await;
                        crate::trace!("dec base");
                        active_count.fetch_sub(1, Ordering::SeqCst);
                    })
                };
                let polling = {
                    let active_count = Arc::clone(&active_count);
                    // a low poll rate is fine, since another task is already running
                    tokio::spawn(async move {
                        crate::trace!("loop");
                        let mut yield_times = 0;
                        loop {
                            // if no more active tasks are running (other than us),
                            // stop polling and go back to blocking wait
                            let ac = active_count.load(Ordering::Acquire);
                            if ac == 0 {
                                break;
                            }
                            crate::trace!("try_recv");
                            match recv.try_recv() {
                                Err(TryRecvError::Empty) => {
                                    tokio::task::yield_now().await;
                                    yield_times += 1;
                                    // to prevent the runtime from always scheduling
                                    // us (since it's not guaranteed)
                                    // if yield_times > 10 {
                                    //     tokio::time::sleep(Duration::from_millis(1)).await;
                                    //     yield_times = 0;
                                    // }
                                }
                                Err(_) => {
                crate::trace!("disconnect");
                                    break;
                                }
                                Ok(fut) => {
                crate::trace!("inc");
                                    active_count.fetch_add(1, Ordering::SeqCst);
                                    let active_count = Arc::clone(&active_count);
                                    tokio::spawn(async move {
                                        fut.await;
                                        crate::trace!("dec");
                                        active_count.fetch_sub(1, Ordering::SeqCst);
                                    });
                                }
                            }
                        }
                        recv
                    })
                };
                let (recv_, _) = tokio::join!(polling, base);
                recv = recv_.expect("monitoring task panicked!");
            }
        });
    });
    LwThread { spawn: send, id: handle.thread().id() }
});

/// Entry point from sync context to run a light weight async job.
///
/// This is only suitable for light weight jobs, most should be IO
/// and not CPU-bound processing. If the job only involves CPU-bound
/// processing, use `rayon` instead. If the job has both IO and heavy
/// CPU work, use [`run_heavy`](crate::run_heavy) (requires the `heavy` feature).
///
/// The job will be running on the background and not block the current thread.
/// Calling [`join`](AsyncHandle::join) will block the thread until the async job
/// is done.
///
/// If you want to run a job and wait for it synchonously, use [`run`]
pub fn spawn<F>(future: F) -> LwHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    if std::thread::current().id() == THREAD.id {
        panic!("cannot spawn light-weight task from light-weight thread itself, use one of co_* function to spawn in current async context, or use tokio::spawn directly.");
    }
    let (send, recv) = oneshot::channel();
    THREAD.spawn.send(Box::pin(async move {
        let _: Result<_, _> = send.send(future.await);
    })).expect("light-weight thread should never be lost because tokio handles panic. This is a bug");
    LwHandle { recv }
}

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime for blocking tasks")
});

/// Run a light weight async job and block the current thread until
/// it's done. 
///
/// If the async work is also heavy on CPU, use [`run_heavy`](crate::run_heavy) to
/// utilize a multi-threaded tokio runtime. If you want
/// the light weight job to run on the background without blocking
/// the current thread, use [`spawn`]
///
/// Note that this is not equivalent to `cu::spawn(...).join()`,
/// as the job is not ran on the worker thread.
pub fn run<F>(future: F) -> F::Output
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    if std::thread::current().id() == THREAD.id {
        panic!("cannot run blocking task on light-weight thread itself, use one of co_* function to spawn in current async context, or use tokio::spawn directly.");
    }
    RUNTIME.block_on(future)
}

/// Join a bunch of async handles by round-robin polling them. 
/// Results are discarded
pub fn join<T: Send + 'static>(handles: Vec<LwHandle<T>>) {
    // run checks thread internally
    run(async move {
        let mut handles = handles;
        while !handles.is_empty() {
            tokio::time::sleep(Duration::from_millis(50)).await;
            handles.retain(|x| {
                matches!(x.try_join(), Ok(None))
            });
        }
    })
}

/// Join a bunch of async handles by round-robin polling them.
/// The result may have `None` if join failed
pub fn join_collect<T: Send + 'static>(handles: Vec<LwHandle<T>>) -> Vec<Option<T>> {
    // run checks thread internally
    run(async move {
        let mut joined = Vec::with_capacity(handles.len());
        let mut out = Vec::with_capacity(handles.len());
        for _ in 0..handles.len() {
            joined.push(false);
            out.push(None);
        }
        let mut join_count = 0;
        while join_count < handles.len() {
            tokio::time::sleep(Duration::from_millis(50)).await;
            for i in 0..handles.len() {
                if !joined[i] {
                    match handles[i].try_join() {
                        Ok(Some(x)) => {
                            join_count += 1;
                            joined[i] = true;
                            out[i] = Some(x)
                        }
                        Err(e) => {
                            join_count += 1;
                            crate::debug!("[{i}] join fail: {e}");
                            joined[i] = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        out
    })
}
