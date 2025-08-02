//! `cu::co::` Coroutine driver
//!
//! This library is designed to have flexible coroutine handling.
//! The program can have:
//! - everything being `async` - typically involving both CPU-bound
//!   work and IO work interwined.
//! - Some IO heavy work that doesn't really involve CPU - for example,
//!   spawning compiler processes and wait for them, or spawning network requests.
//! - Heavy CPU work that only has a little IO.
//!
//! You pick the execution model you want.
//!
//! # No `async` at all
//! Great - no extra learning is needed. You can just write a normal
//! Rust program. Note that this crate still uses threads to handle
//! progress bar animation and prompting. However, these threads are spawned
//! on-demand and only lasts when they are active (i.e. when there
//! is a progress bar being displayed, or when there is a prompt).
//!
//! # Light-weight `async` + (potentially) heavy CPU work
//! To enable coroutines for light-weight `async` tasks, enable the `coroutine`
//! feature. In this mode, coroutines are drived by a single-threaded `tokio` runtime,
//! and typically, the `async` things are hidden away by synchronous APIs.
//!
//! For example, if you want to spawn 2 processes, and pipe their output:
//! ```rust,ignore
//!
//! let git = cu::bin::which("git").unwrap();
//! let child1 = git.command()
//!     .args(["clone", "https://example1.git", "dest1", "--progress"])
//!     .stdin(cu::cio::raw_inherit())
//!     .stdboth(cu::cio::spinner("cloning example1").info())
//!     .spawn()?;
//! let child2 = git.command()
//!     .args(["clone", "https://example2.git", "dest2", "--progress"])
//!     .stdin(cu::cio::raw_inherit())
//!     .stdboth(cu::cio::spinner("cloning example2").info())
//!     .spawn()?;
//!
//! // child1 and child2 are now both running, their IO are handled
//! // asynchronously on the same IO thread, driven by the same single-threaded
//! // tokio runtime. The print animation thread handles the progress spinners
//!
//! // we need both to finish, so doesn't matter if we wait for them concurrently,
//! // since they are executed on the background anyway
//! child1.wait_nz()?; // this checks non-zero exit status
//! child2.wait_nz()?;
//! ```
//!
//! The benefit of this is we don't waste resource with a potentially heavy,
//! multi-threaded async runtime. This means you can do heavy CPU-bound work
//! parallely with max efficiency, with something like `rayon`.
//!
//! You can also interact with the async thread directly, with [`cu::co::spawn`](`crate::co::spawn`)
//! and [`cu::co::run`](crate::co::run). These will post the work to run on the async thread,
//! and give you a blocking join handle. Note that in this mode, you will get a panic
//! if you try to spawn or join while on the IO thread. In those cases, you should
//! use `tokio::spawn` to spawn the task, or use the async APIs that does that under the hood
//!
//! ```rust,ignore
//! // suppose we are in an async context, and we don't want to block
//! let git = cu::bin::which("git").unwrap();
//! let child1 = git.command()
//!     .args(["clone", "https://example1.git", "dest1", "--progress"])
//!     .stdin(cu::cio::raw_inherit())
//!     .stdboth(cu::cio::spinner("cloning example1").info())
//!     .co_spawn()?; // replace `spawn()` with `co_spawn()`
//! let child2 = git.command()
//!     .args(["clone", "https://example2.git", "dest2", "--progress"])
//!     .stdin(cu::cio::raw_inherit())
//!     .stdboth(cu::cio::spinner("cloning example2").info())
//!     .co_spawn()?;
//!
//! // co_spawn will give a version of the child where the tasks
//! // are spawned on the current tokio runtime (with tokio::spawn),
//! // and subsequent operations are async
//! child1.wait_nz().await?;
//! child2.wait_nz().await?;
//! ```
//!
//! # Heavy `async` work
//! If there are a lot of async and sync work interwined, it might make
//! sense to bring in the heavy, multi-threaded `tokio` runtime.
//!
//! The `coroutine-heavy` feature is needed, which automatically turns on
//! the `coroutine` feature as well. (Note that you don't need the `tokio::main`
//! macro - the `co_run` function handles that for you).
//!
//! ```rust,ignore
//!
//! fn main() -> std::process::ExitStatus {
//!     // This spawns `main_internal` on a multi-threaded `tokio` runtime
//!     cu::cli::co_run(main_internal)
//! }
//!
//! async fn main_internal(args: cu::cli::Flags) -> cu::Result<()> {
//!     tokio::spawn(async move { /*...*/ })
//! }
//! ```
//!
//! You can also use a synchronous entry point, and spawn the heavy
//! tasks manually, which gives you more control over when to engage
//! the heavy runtime
//!
//! ```rust,ignore
//! fn main() -> std::process::ExitStatus {
//!     cu::cli::run(main_internal)
//! }
//! fn main_internal(args: cu::cli::Flags) -> cu::Result<()> {
//!     // this internally calls `block_on` to enter the tokio runtime
//!     cu::co::run_heavy(async move { /*...*/ });
//!     // and you can do it multiple times to switch between sync and async
//!     cu::co::run_heavy(async move { /*...*/ });
//! }
//! ```
//!
//! Note that there is no `spawn_heavy` - you should just use `tokio::spawn`.
//! The single-threaded functions are not disabled, but are disencouraged
//! and you should use the `co_*` functions if possible when inside a heavy
//! runtime context, to fully take advantage of all the resources in the runtime.

#[cfg(feature="coroutine")]
pub use crate::async_::{
    JoinHandle, LwHandle, Pool, spawn, run, join, join_collect};
#[cfg(feature="coroutine-heavy")]
pub use crate::async_::run_heavy;
