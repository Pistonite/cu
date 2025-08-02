use std::pin::Pin;

mod light;
pub use light::*;
#[cfg(feature="coroutine-heavy")]
mod heavy;
#[cfg(feature="coroutine-heavy")]
pub use heavy::*;

mod pool;
pub use pool::*;



/// Alias for a boxed future
pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
pub type JoinHandle<T> = tokio::task::JoinHandle<T>;
pub type JoinError = tokio::task::JoinError;

