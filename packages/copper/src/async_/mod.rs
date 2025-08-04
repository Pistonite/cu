use std::pin::Pin;

#[cfg(feature = "coroutine")]
mod pool;
#[cfg(feature = "coroutine")]
pub use pool::*;

#[cfg(feature = "coroutine")]
mod runtime;
#[cfg(feature = "coroutine")]
pub use runtime::*;

/// Alias for a boxed future
pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
