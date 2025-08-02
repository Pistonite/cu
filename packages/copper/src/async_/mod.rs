use std::pin::Pin;

mod light;
pub use light::*;
#[cfg(feature="heavy")]
mod heavy;
#[cfg(feature="heavy")]
pub use heavy::*;

mod pool;
pub use pool::*;



/// Alias for a boxed future
pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Handle for an async task
pub struct AsyncHandle<T> {
    recv: oneshot::Receiver<T>,
}
impl<T> AsyncHandle<T> {
    /// Block the current thread on joining the handle. This should NOT
    /// be done from async context
    pub fn join(self) -> crate::Result<T> {
        use crate::Context as _;
        self.recv.recv().context("failed to join an async handle")
    }

    pub fn try_join(&self) -> crate::Result<Option<T>> {
        use crate::Context as _;
        match self.recv.try_recv() {
            Ok(x) => Ok(Some(x)),
            Err(oneshot::TryRecvError::Empty) => Ok(None),
            Err(e) => Err(e).context("failed to join an async handle")
        }
    }
}
