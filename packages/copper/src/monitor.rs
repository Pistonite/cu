use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::{Arc, LazyLock, atomic::AtomicUsize};
use std::time::Duration;

use tokio::runtime::Builder;

type BoxedFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

struct Monitor {
    send: Sender<BoxedFuture>,
}

pub(crate) struct JoinHandle<T> {
    recv: oneshot::Receiver<T>
}
impl<T> JoinHandle<T> {
    pub fn join(self) -> Option<T> {
        self.recv.recv().ok()
    }
}

/// The tokio runtime instance
static MONITOR: LazyLock<Monitor> = LazyLock::new(|| {
    // we only need one thread since the tasks are lightweight
    let (send, recv) = mpsc::channel();
    std::thread::spawn(move || {
        let runtime = Builder::new_current_thread()
            .enable_all().build().expect("failed to build tokio runtime for monitoring tasks");
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
                                Err(TryRecvError::Empty) => {
                                    tokio::time::sleep(POLL_INTERVAL).await
                                }
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
    Monitor { send }
});

/// Spawn a monitoring task. This is crate-internal to avoid
/// heavy work to be scheduled
pub(crate) fn spawn<F>(future: F) -> JoinHandle<F::Output>
where F: Future + Send + 'static, F::Output: Send + 'static
{
    let (send, recv) = oneshot::channel();
    let _ = MONITOR.send.send(Box::pin(async move {
        let _: Result<_, _> = send.send(future.await);
    }));
    JoinHandle { recv }
}
