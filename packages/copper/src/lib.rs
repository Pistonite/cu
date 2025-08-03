//! Batteries-included common utils
//!
//! # General Principal
//! `cu` tries to be as short as possible with imports. Common and misc
//! utilities are exported directly by the crate and should be used
//! as `cu::xxx` directly. Sub-functionalities are bundled when makes
//! sense, and should be called from submodules directly, like `cu::fs::xxx`
//! or `cu::co::xxx`. The submodules are usually 2-4 characters.
//!
//! The only time to use `use` to import from `cu`, is with the prelude module
//! `pre`:
//! ```rust
//! use cu::pre::*;
//! ```
//! This imports traits like [`Context`] and [`PathExtension`] into scope.
//!
//! Quick Feature Reference:
//! - `cli`: Enables CLI entry points and integration with `clap`
//! - `prompt`: Enables macros to show prompt to the user
//! - `fs`: Enables file system utils
//! - `coroutine` and `coroutine-heavy`: Enables `async` and integration with `tokio`
//! - `process`: Enables utils spawning child process
//!
//! # CLI
//! Enable the `cli` feature when using `cu` in a binary crate.
//! See [`cli`](module@cli) for usage. This also takes care of setting
//! up logging.
//!
//! Note that general usage of logging, prompting, and progress bar does not require
//! the `cli` flag, meaning libraries can also use them.
//!
//! # `log` integration
//! In additional to the common `error`, `warn`, `info`, `debug`, `trace`
//! log types, `cu` provides 2 extra types:
//! - `print`: like `info`, but has a higher importance
//! - `hint`: like `print`, but specifically for hinting actions the user can take
//!   (to resolve an error, for example).
//!
//! These 2 levels are not directly controlled by `log`,
//! and can still print when logging is statically disabled.
//!
//! The following table shows what are printed for each level,
//! (other than `print` and `hint`, the rest are re-exports from `log`)
//! |         | `-qq` | ` -q` | `   ` | ` -v` | `-vv` |
//! |-|-      |-     |-       |-     |-      |
//! | [`error!`](crate::error) | ❌ | ✅ | ✅ | ✅ | ✅ |
//! | [`hint!`](crate::hint) | ❌ | ✅ | ✅ | ✅ | ✅ |
//! | [`print!`](macro@crate::print) | ❌ | ✅ | ✅ | ✅ | ✅ |
//! | [`warn!`](crate::warn)  | ❌ | ❌ | ✅ | ✅ | ✅ |
//! | [`info!`](crate::info)  | ❌ | ❌ | ✅ | ✅ | ✅ |
//! | [`debug!`](crate::debug) | ❌ | ❌ | ❌ | ✅ | ✅ |
//! | [`trace!`](crate::trace) | ❌ | ❌ | ❌ | ❌ | ✅ |
//!
//! The `RUST_LOG` environment variable is also supported in the same
//! way as in [`env_logger`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging).
//! When mixing `RUST_LOG` and verbosity flags, logging messages are filtered
//! by `RUST_LOG`, and the verbosity would only apply to `print` and `hint`
//!
//! When setting up test, you can use [`log_init`] to quickly inititialize logging
//! without dealing with the details.
//!
//! [`set_thread_print_name`] can be used to add a prefix to all messages printed
//! by the current thread.
//!
//! Messages that are too long and multi-line messages are automatically wrapped.
//!
//! # Progress Bar
//! Animated progress bars are displayed at the bottom of the terminal.
//! While progress bars are visible, printing still works and will be put
//! above the bars. However, prints will be buffered and refreshed
//! and the same frame rate as the bars.
//!
//! [`progress_bar`] and [`progress_bar_lowp`] are used to create a bar.
//! The only difference is that `lowp` doesn't print a message when the progress
//! is done (as if the bar was never there). The bar takes a message to indicate
//! the current action, and each update call can accept a message to indicate
//! the current step. When `bar` is dropped, it will print a done message.
//!
//! ```rust,no_run
//! use std::time::Duration;
//! {
//!    let bar = cu::progress_bar(10, "This takes 2.5 seconds");
//!    for i in 0..10 {
//!        cu::progress!(&bar, i, "step {i}");
//!        cu::debug!("this is debug message");
//!        std::thread::sleep(Duration::from_millis(250));
//!    }
//! }
//! ```
//!
//! [`progress_unbounded`] and [`progress_unbounded_lowp`] are variants
//! that doesn't display the total steps. Use `()` as the step placeholder
//! when updating the bar.
//!
//! # Prompting
//! With the `prompt` feature enabled, you can
//! use [`prompt!`] and [`yesno!`] to show prompts.
//!
//! The prompts are thread-safe, meaning
//! You can call them from multiple threads, and they will be queued to prompt the user one after
//! the other. Prompts are always shown regardless of verbosity. But when stdout is redirected,
//! they will not render in terminal.
//!
//! # Async coroutines
//! See [`cu::co`](co) to see how coroutine and `async` works in this crate.
//!
//! # File, Path and Process
//! The `fs` feature enables [`cu::fs`](fs) and [`cu::bin`](bin), as well
//! as additional path helpers via [`PathExtension`].
//!
//! The file system wrappers provided by `cu` has automatic error logging,
//! so you can freely `?` without running into errors like this without any context:
//! ```txt
//! Error: system cannot find the specified path
//! ```
//!
//! The `process` feature enables spawning child process and integrate IO with the child
//! into the printing utils in this crate. See [`CommandBuilder`] for more info.

#![cfg_attr(any(docsrs, feature = "nightly"), feature(doc_auto_cfg))]

// mod env_var;
// pub use env_var::*;
// mod parse;
// pub use parse::*;
//
// mod monitor;

mod async_;
pub use async_::BoxedFuture;
#[cfg(feature="coroutine")]
pub mod co;

#[cfg(feature="process")]
mod process;
#[cfg(feature="process")]
pub use process::{pio, CommandBuilder, CommandBuilderDefault};

#[cfg(feature="fs")]
pub mod bin;
#[cfg(feature="fs")]
#[doc(inline)]
pub use bin::which;

/// File System utils (WIP)
#[cfg(feature="fs")]
pub mod fs;

/// Path utils
#[cfg(feature="fs")]
mod path;
#[cfg(feature="fs")]
pub use path::PathExtension;

#[cfg(feature="cli")]
pub mod cli;
#[cfg(feature="cli")]
pub use copper_proc_macros::cli;

/// Low level printing utils and integration with log and clap
mod print;
pub use print::{
    ColorLevel, PrintLevel, ProgressBar, PromptLevel, color_enabled, init_print_options,
    progress_bar, progress_bar_lowp, progress_unbounded, progress_unbounded_lowp,
    set_thread_print_name, log_enabled,
    term_width, term_width_or_max, term_width_height,log_init
};

/// Level shorthand for message/events
pub mod lv {
    /// Error
    pub const E: crate::__priv::Lv = crate::__priv::Lv::Error;
    /// Hint
    pub const H: crate::__priv::Lv = crate::__priv::Lv::Hint;
    /// Print
    pub const P: crate::__priv::Lv = crate::__priv::Lv::Print;
    /// Warn
    pub const W: crate::__priv::Lv = crate::__priv::Lv::Warn;
    /// Info
    pub const I: crate::__priv::Lv = crate::__priv::Lv::Info;
    /// Debug
    pub const D: crate::__priv::Lv = crate::__priv::Lv::Debug;
    /// Trace
    pub const T: crate::__priv::Lv = crate::__priv::Lv::Trace;
}

// Atomic helpers
mod atomic;
pub use atomic::*;

// re-exports from libraries
pub use anyhow::{Context, Result, bail, ensure, Ok};
pub use log::{debug, error, info, trace, warn};

#[doc(hidden)]
pub mod __priv {
    #[cfg(feature="process")]
    pub use crate::process::__ConfigFn;
    pub use crate::print::{__print_with_level, __prompt, __prompt_yesno, Lv};
}

/// Prelude imports
pub mod pre {
    pub use crate::Context as _;
    #[cfg(feature="fs")]
    pub use crate::PathExtension as _;
}
