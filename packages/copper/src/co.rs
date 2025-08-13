//! `cu::co::` Coroutine driver
//!
//! This library is designed to have flexible coroutine handling,
//! being able to handle `async` both on the current thread,
//! and on one or more background threads.
//!
//! For example, consider these program styles:
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
//! # Async entry point
//!
//! With the [`cli`](module@crate::cli) module, you can use the same macro
//! for an async entry point
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
//! Note that the entry point is still drived by the main thread despite being `async`
//! (even if `coroutine-heavy` feature is enabled), meaning that the above program
//! is still single-threaded! This makes sense because the (fake) workload doesn't benefit
//! at all from having multiple threads.
//!
//! By default, the number of background threads is 1.
//! Enabling the `coroutine-heavy` feature will change it
//! to the number of processors.
//!
//! # Internal Coroutine
//! Some `cu` functions use coroutines internally behind "synchronous" APIs,
//! allowing seamless integration from a synchronous context.
//!
//! For example, `cu` uses coroutines to drive inputs and outputs from a command:
//! ```rust,no_run
//! # use pistonite_cu as cu;
//! use cu::pre::*;
//!
//! #[cu::cli]
//! fn main(_: cu::cli::Flags) -> cu::Result<()> {
//!     let git = cu::which("git")?;
//!     let child1 = git.command()
//!         .args(["clone", "https://example1.git", "dest1", "--progress"])
//!         .stdin_null()
//!     // use a progress bar to display progress, and print other
//!     // messages as info
//!         .stdoe(cu::pio::spinner("cloning example1").info())
//!         .spawn()?;
//!     // same configuration
//!     let child2 = git.command()
//!         .args(["clone", "https://example2.git", "dest2", "--progress"])
//!         .stdin_null()
//!         .stdoe(cu::pio::spinner("cloning example2").info())
//!         .spawn()?;
//!    
//!     // Both childs are now running as separate processes in the OS.
//!     // Also, IO from both childs are drived by the same background thread.
//!     // You can block the main thread to do other work, and it will not
//!     // block the child from printing messages
//!    
//!     // since we don't get benefit from one child finishing early
//!     // here, we just wait for them in order
//!     child1.wait_nz()?;
//!     child2.wait_nz()?;
//!    
//!     Ok(())
//! }
//! ```
//!
//! # `co_*` APIs
//! Many APIs in `cu` has a same version with `co_` prefix.
//! These are designed to be called when you are already in an asynchronous
//! context. For example, we can rewrite the example above using `co_wait_nz`.
//! Note that in this case, there's no benefit of using `co_spawn`/`co_wait_nz`,
//! since we are not doing any extra work.
//!
//! ```rust,no_run
//! # use pistonite_cu as cu;
//! use cu::pre::*;
//! #[cu::cli]
//! async fn main(_: cu::cli::Flags) -> cu::Result<()> {
//!     let git = cu::which("git")?;
//!     let child1 = git.command()
//!         .args(["clone", "https://example1.git", "dest1", "--progress"])
//!         .stdin_null()
//!         .stdoe(cu::pio::spinner("cloning example1").info())
//!         // using co_spawn() will do the work needed at spawn time
//!         // using the current async context, instead of off-loading
//!         // it to a background thread.
//!         .co_spawn().await?;
//!         // however, note that the IO work, once spawned, are still
//!         // driven by a background thread regardless of which spawn API
//!         // is used
//!     // same configuration
//!     let child2 = git.command()
//!         .args(["clone", "https://example2.git", "dest2", "--progress"])
//!         .stdin_null()
//!         .stdoe(cu::pio::spinner("cloning example2").info())
//!         .co_spawn().await?;
//!    
//!     child1.co_wait_nz().await?;
//!     child2.co_wait_nz().await?;
//!    
//!     Ok(())
//! }
//! ```

pub use crate::async_::{AbortHandle, Handle, Pool, RobustAbortHandle, RobustHandle, run, spawn};

#[cfg(not(feature = "coroutine-heavy"))]
pub use crate::async_::block;
#[cfg(feature = "coroutine-heavy")]
pub use crate::async_::spawn_blocking;
