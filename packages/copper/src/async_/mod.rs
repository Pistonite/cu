use std::pin::Pin;

mod pool;
pub use pool::*;

mod runtime;
pub use runtime::*;

/// Alias for a boxed future
pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

