use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::{Arc, LazyLock, atomic::AtomicUsize};
use std::time::Duration;

use tokio::runtime::{Builder, Runtime};

use super::{BoxedFuture, AsyncHandle};

/// A light weight worker thread that handles async tasks
/// using a single-threaded tokio runtime
static SPAWNER: LazyLock<Sender<BoxedFuture<()>>> = LazyLock::new(|| {
    // we only need one thread since the tasks are lightweight
    let (send, recv) = mpsc::channel();
    std::thread::spawn(move || {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime for async light weight tasks");
        // runtime loop
        runtime.block_on(async move {
            let active_count = Arc::new(AtomicUsize::new(0));
            let mut recv = recv;
            loop {
                // Block the entire runtime, until we get a task
                let Ok(fut) = recv.recv() else {
                    break;
                };
                active_count.fetch_add(1, Ordering::SeqCst);
                let base = tokio::spawn(fut);
                let polling = {
                    let active_count = Arc::clone(&active_count);
                    // a low poll rate is fine, since another task is already running
                    const POLL_INTERVAL: Duration = Duration::from_millis(100);
                    tokio::spawn(async move {
                        loop {
                            // if no more active tasks are running (other than us),
                            // stop polling and go back to blocking wait
                            if active_count.load(Ordering::Acquire) == 0 {
                                break;
                            }
                            match recv.try_recv() {
                                Err(TryRecvError::Empty) => tokio::time::sleep(POLL_INTERVAL).await,
                                Err(_) => break,
                                Ok(fut) => {
                                    active_count.fetch_add(1, Ordering::SeqCst);
                                    let active_count = Arc::clone(&active_count);
                                    tokio::spawn(async move {
                                        fut.await;
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
    send
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
pub fn spawn<F>(future: F) -> AsyncHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (send, recv) = oneshot::channel();
    let _ = SPAWNER.send(Box::pin(async move {
        let _: Result<_, _> = send.send(future.await);
    }));
    AsyncHandle { recv }
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
    RUNTIME.block_on(future)
}
