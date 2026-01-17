//! # Cu = Copper
//! Batteries-included common utils
//!
//! (If you are viewing this on docs.rs, please use the [self-hosted
//! version](https://cu.pistonite.dev) instead)
//!
//! # Quick start
//! When installing, rename the crate to `cu` in `Cargo.toml`:
//! ```toml
//! # Cargo.toml
//! # ...
//! [dependencies.cu]
//! package = "pistonite-cu"
//! version = "..." # check by running `cargo info pistonite-cu`
//! features = [ "full" ] # see docs
//!
//! [dependencies]
//! # ...
//! ```
//!
//! The goal with using `cu` is use only one `use` statement:
//! ```rust
//! # use pistonite_cu as cu;
//! use cu::pre::*;
//! ```
//! This brings into scope a few things:
//! - Traits that are expected to be used, such as `anyhow::Context`
//! - Re-exports of modules, such as `json` if the `json` feature is enabled
//!
//! If a function or type is not included with `pre::*`, that means the canonical
//! style for using it is with the full path, for example, you would write:
//!
//! ```rust,ignore
//! # use pistonite_cu as cu;
//! use cu::pre::*;
//!
//! fn read_file() -> cu::Result<()> {
//!     cu::fs::read_string("foo/bar.txt")?;
//!     cu::info!("successfully read file");
//!     Ok(())
//! }
//! ```
//!
//! instead of the below:
//! ```rust,ignore
//! # use pistonite_cu as cu;
//! use cu::pre::*;
//! use cu::{Result, fs, info};
//! // ^ don't include extra uses!
//! //   the biggest disadvantage of this is
//! //   it's easy to confuse with types in the standard library
//!
//! fn read_file() -> Result<()> {
//!     fs::read_string("foo/bar.txt")?;
//!     info!("successfully read file");
//!     Ok(())
//! }
//! ```
//!
//! # Quick Reference
//! - [Error Handling](macro@crate::check) (via [`anyhow`](https://docs.rs/anyhow))
//! - [Logging](mod@crate::lv) (via [`log`](https://docs.rs/log))
//! - [Printing and Command Line Interface](mod@crate::cli) (CLI arg parsing via
//!   [`clap`](https://docs.rs/clap))
//! - [Handling Ctrl-C](fn@crate::cli::ctrlc_frame)
//! - [Progress Bars](fn@crate::progress)
//! - [Prompting](macro@crate::prompt)
//! - [Coroutines (Async)](mod@crate::co) (via [`tokio`](https://docs.rs/tokio))
//! - [File System Paths and Strings](trait@crate::str::PathExtension)
//! - [File System Operations](mod@crate::fs)
//! - [Binary Path Registry](mod@crate::fs::bin)
//! - [Spawning Child Processes](crate::Command)
//! - [Parsing](trait@Parse) (via [`serde`](https://docs.rs/serde))
//! - Derive Macros: the `derive` feature, via [`derive_more`](https://docs.rs/derive_more)

#![cfg_attr(any(docsrs, feature = "nightly"), feature(doc_cfg))]

// for macros
extern crate self as cu;

// --- Basic stuff (no feature needed) ---
pub mod str;
pub use str::{ByteFormat, ZString};

mod env_var;
pub use env_var::*;
mod atomic; // Atomic helpers
pub use atomic::*;
mod misc; // other stuff that doesn't have a place
pub use misc::*;

// --- Error Handling (no feature needed) ---
mod errhand;
pub use errhand::*;
pub use pistonite_cu_proc_macros::context;

// --- Logging (no feature needed) ---
pub mod lv;
pub use lv::{debug, error, info, trace, warn};

// --- Command Line Interface (print/cli/prompt/prompt-password feature) ---
#[cfg(feature = "print")]
pub mod cli;
#[cfg(feature = "prompt-password")]
pub use cli::password_chars_legal;
#[cfg(feature = "print")]
pub use cli::{ProgressBar, ProgressBarBuilder, progress};
#[cfg(feature = "cli")]
pub use pistonite_cu_proc_macros::cli;

// --- Async (coroutine/coroutine-heavy) ---
/// Alias for a boxed future
pub type BoxedFuture<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'static>>;
#[cfg(feature = "coroutine")]
pub mod co;
#[cfg(feature = "coroutine")]
pub use co::{join, select, try_join};

// --- File System ---
#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "fs")]
pub use fs::bin;
#[cfg(feature = "fs")]
pub use fs::bin::which;

// === above is refactored and documented ===

// --- Child Process ---
#[cfg(feature = "process")]
mod process;
#[cfg(feature = "process")]
pub use process::{Child, Command, CommandBuilder, Spawn, color_flag, color_flag_eq, pio};
#[cfg(all(feature = "process", feature = "print"))]
pub use process::{width_flag, width_flag_eq};

/// Parsing utilities
#[cfg(feature = "parse")]
mod parse;
#[cfg(feature = "parse")]
pub use parse::*;
#[cfg(feature = "parse")]
pub use pistonite_cu_proc_macros::Parse;

#[doc(hidden)]
pub mod __priv {
    #[cfg(feature = "process")]
    pub use crate::process::__ConfigFn;
}

/// Lib re-exports
#[doc(hidden)]
pub mod lib {
    #[cfg(feature = "cli")]
    pub use clap;
    #[cfg(feature = "derive")]
    pub use derive_more;
}

/// Prelude imports
#[doc(hidden)]
pub mod pre {
    pub use crate::Context as _;
    pub use crate::str::{OsStrExtension as _, OsStrExtensionOwned as _};

    #[cfg(feature = "cli")]
    pub use crate::lib::clap;

    #[cfg(feature = "coroutine")]
    pub use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _};

    #[cfg(feature = "fs")]
    pub use crate::str::PathExtension as _;

    #[cfg(feature = "parse")]
    pub use crate::ParseTo as _;
    #[cfg(feature = "process")]
    pub use crate::Spawn as _;
    #[cfg(feature = "json")]
    pub use crate::json;
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
}
