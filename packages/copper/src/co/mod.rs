//! # Coroutines (Async)
//!
//! `cu` is designed to have flexible coroutine handling. For example, consider these program styles:
//! - everything being `async` - typically involving both CPU-bound
//!   work and IO work interwined. Can take advantage of multiple background threads.
//! - Some IO heavy work that doesn't really involve CPU - for example,
//!   spawning compiler processes and wait for them, or spawning network requests.
//!   Usually won't have significant performance benefit from having multiple background threads.
//! - Heavy CPU work that only has a little IO. Using `async` usually has very little
//!   benefit. (Would probably use something like `rayon` to get parallelism).
//!
//! You pick the style you want.
//!
//! The async runtime being used under the hood is [`tokio`](https://docs.rs/tokio).
//! There are 2 feature flags you can choose from: `coroutine` and `coroutine-heavy`.
//! `coroutine` uses one foreground (current-thread) tokio runtime and one background thread to
//! drive IO tasks. `coroutine-heavy` does not have a current-thread runtime - everything
//! is done on the multi-threaded, background runtime.
//!
//! # Async entry point
//!
//! To make your entire program async with [`cli`](module@crate::cli),
//! simply make the `main` function `async.
//!
//! ```rust
//! use std::time::Duration;
//! # use pistonite_cu as cu;
//! #[cu::cli]
//! async fn main(_: cu::cli::Flags) -> cu::Result<()> {
//!     cu::info!("doing some work");
//!     tokio::time::sleep(Duration::from_millis(100)).await;
//!     cu::info!("done");
//!     Ok(())
//! }
//! ```
//!
//! When using `coroutine`, the main future will be spawned onto the current-thread
//! runtime (so the main thread is still driving it). When using `coroutine-heavy`,
//! the main future is spawned onto the background runtime, and the main thread
//! waits until the future is completed.
//!
//! # Coroutines used internally and `co_*` APIs
//! Some `cu` functions use coroutines internally behind "synchronous" APIs,
//! allowing seamless integration from a synchronous context.
//!
//! For example, when spawning child processes, an async task processes
//! IO from the child and streams results to the main thread. This allows
//! for a clean API to for example, read child's output line-by-line.
//!
//! However, there is an important catch - it is crucial that we never block
//! an async runtime. This means to wait for a future:
//! - If we are not in an async runtime, we can enter the async runtime by
//!   calling an entry point to the runtime (a.k.a `block`), to block
//!   the current thread while letting the runtime run until the future is finished.
//! - If we are already in an async runtime, we must call `.await` instead
//!   of block. Otherwise, either the entire runtime will block and may deadlock,
//!   or tokio will detect it and panic.
//!
//! This is why the APIs that use `coroutine` under the hood will have
//! another version with a `co_*` prefix. For example, `spawn` and `co_spawn`.
//! You MUST use `.spawn()?` if not in an async runtime, and `.co_spawn().await?`
//! in an async runtime. Note that calling `.co_spawn()` while not in an async runtime
//! is also not allowed, because tokio will assume a runtime is active and panic
//! if not.
//!
//! # Advanced Usage
//! If additional functionality from `tokio` is needed (not already provided by re-exports),
//! then you can add `tokio` to `Cargo.toml`:
//! ```toml
//! [dependencies]
//! tokio = "1"
//! ```

// re-exports
pub use tokio::{join, select, try_join};

mod runtime;
#[cfg(not(feature = "coroutine-heavy"))]
pub use runtime::block;
#[cfg(feature = "coroutine-heavy")]
pub use runtime::spawn_blocking;
pub use runtime::{run, spawn};

mod pool;
pub use pool::{Pool, Set, pool, set, set_flatten};

mod handle;
pub use handle::{AbortHandle, Handle, RobustAbortHandle, RobustHandle};
mod co_util;
