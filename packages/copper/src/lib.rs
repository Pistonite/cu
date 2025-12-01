//! Batteries-included common utils
//!
//! (If you are viewing this on docs.rs, please use the [self-hosted
//! version](https://cu.pistonite.dev) instead)
//!
//! # Install
//! Since crates.io does not have namespaces, this crate has a prefix.
//! You should manually rename it to `cu`, as that's what the proc-macros
//! expect.
//! ```toml
//! # Cargo.toml
//! # ...
//! [dependencies.cu]
//! package = "pistonite-cu"
//! version = "..." # check by running `cargo info pistonite-cu`
//! features = [ "full" ] # see docs
//!
//! # ...
//! [dependencies]
//! ```
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
//! # use pistonite_cu as cu;
//! use cu::pre::*;
//! ```
//! This imports traits like [`Context`] and [`PathExtension`] into scope.
//!
//! # Feature Reference:
//! - `cli`, `print`, `prompt`:
//!   See [`cli`](module@cli). Note that logging is still available without any feature flag.
//! - `coroutine` and `coroutine-heavy`:
//!   Enables `async` and integration with `tokio`. See [`cu::co`](module@co).
//! - `fs`: Enables file system utils. See [`cu::fs`](module@fs) and [`cu::bin`](module@bin).
//! - `process`: Enables utils spawning child process. See [`Command`].
//! - `parse`, `json`, `yaml`, `toml`:
//!   Enable parsing utilities, and additional support for common formats. See
//!   [`Parse`](trait@Parse).

#![cfg_attr(any(docsrs, feature = "nightly"), feature(doc_cfg))]

#[cfg(feature = "process")]
mod process;
#[cfg(feature = "process")]
pub use process::{Child, Command, CommandBuilder, Spawn, color_flag, color_flag_eq, pio};
#[cfg(all(feature = "process", feature = "print"))]
pub use process::{width_flag, width_flag_eq};

#[cfg(feature = "fs")]
pub mod bin;
#[cfg(feature = "fs")]
#[doc(inline)]
pub use bin::which;

/// File System utils (WIP)
#[cfg(feature = "fs")]
pub mod fs;

/// Path utils
#[cfg(feature = "fs")]
mod path;
#[cfg(feature = "fs")]
pub use path::{PathExtension, PathExtensionOwned};

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub use pistonite_cu_proc_macros::cli;

#[cfg(feature = "coroutine")]
mod async_;
/// Alias for a boxed future
pub type BoxedFuture<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'static>>;
#[cfg(feature = "coroutine")]
pub mod co;

/// Low level printing utils and integration with log and clap
#[cfg(feature = "print")]
mod print;
#[cfg(feature = "prompt-password")]
pub use print::check_password_legality;
#[cfg(feature = "print")]
pub use print::{
    ProgressBar, ZeroWhenDropString, init_print_options, log_init, progress_bar, progress_bar_lowp,
    progress_unbounded, progress_unbounded_lowp, set_thread_print_name, term_width,
    term_width_height, term_width_or_max,
};

/// Printing level values
pub mod lv;
#[doc(inline)]
pub use lv::{color_enabled, disable_print_time, disable_trace_hint, log_enabled};

/// Parsing utilities
#[cfg(feature = "parse")]
mod parse;
#[cfg(feature = "parse")]
pub use parse::*;
#[cfg(feature = "parse")]
pub use pistonite_cu_proc_macros::Parse;
mod env_var;
pub use env_var::*;

// Atomic helpers
mod atomic;
pub use atomic::*;

// other stuff that doesn't have a place
mod misc;
pub use misc::*;

// re-exports from libraries
pub use anyhow::{Context, Error, Ok, Result, anyhow as fmterr, bail, ensure};
pub use log::{debug, error, info, trace, warn};
#[cfg(feature = "coroutine")]
pub use tokio::{join, try_join};

#[doc(hidden)]
pub mod __priv {
    #[cfg(feature = "print")]
    pub use crate::print::{__print_with_level, __prompt, __prompt_yesno};
    #[cfg(feature = "process")]
    pub use crate::process::__ConfigFn;
}

/// Lib re-exports
pub mod lib {
    #[cfg(feature = "cli")]
    pub use clap;
    #[cfg(feature = "derive")]
    pub use derive_more;
}

/// Prelude imports
pub mod pre {
    pub use crate::Context as _;
    #[cfg(feature = "parse")]
    pub use crate::ParseTo as _;
    #[cfg(feature = "fs")]
    pub use crate::PathExtension as _;
    #[cfg(feature = "fs")]
    pub use crate::PathExtensionOwned as _;
    #[cfg(feature = "process")]
    pub use crate::Spawn as _;
    #[cfg(feature = "json")]
    pub use crate::json;
    #[cfg(feature = "cli")]
    pub use crate::lib::clap;
    #[cfg(feature = "toml")]
    pub use crate::toml;
    #[cfg(feature = "yaml")]
    pub use crate::yaml;
    #[cfg(feature = "serde")]
    pub use ::serde::{Deserialize, Serialize};

    #[cfg(feature = "derive")]
    pub use crate::lib::derive_more;
    #[cfg(feature = "derive")]
    pub use crate::lib::derive_more::{
        AsMut, AsRef, Binary as DisplayBinary, Constructor, Debug as DebugCustom, Deref, DerefMut,
        Display, From, Index, IndexMut, Into, IntoIterator, IsVariant, LowerExp as DisplayLowerExp,
        LowerHex as DisplayLowerHex, Octal as DisplayOctal, Pointer as DisplayPointer,
        UpperExp as DisplayUpperExp, UpperHex as DisplayUpperHex,
    };

    #[cfg(feature = "coroutine")]
    pub use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _};
}
