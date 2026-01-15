//! # Printting and Command Line Interface
//!
//! There are 4 feature flags related to CLI
//! - `print`: This is the most minimal feature set. Using
//!   features from this feature flag means you acknowledge your code
//!   is being called from a program that uses `cu::cli` (i.e. the `cli` feature)
//! - `cli`: Use this if your crate is the end binary (i.e. not a library).
//!   This integrates and re-exports [`clap`](https://docs.rs/clap).
//!   - This turns on `print` automatically
//! - `prompt`: This implies `print` and will also enable the ability to show prompts in the terminal.
//! - `prompt-password`: This implies `prompt` (which implies `print`), and allows prompting for
//!   password (which hides the input when user types into the terminal)
//!
//! # Integration with `clap`
//!
//! When the `cli` feature is enabled, `clap` is re-exported from the prelude,
//! so you can use `clap` as if it's a dependency, without actually adding
//! it to your `Cargo.toml`
//! ```rust,no_run
//! # use pistonite_cu as cu;
//! use cu::pre::*;
//! use clap::Parser;
//!
//! #[derive(Parser)]
//! struct MyCli {
//!     /// Just an example flag
//!     #[clap(short, long)]
//!     hello: bool,
//! }
//! ```
//!
//! # Common Command Options
//! The [`Flags`] struct implement `clap::Args` to provide common
//! options that integrates with the rest of the crate:
//! - `--verbose`/`-v` to increase verbose level.
//! - `--quiet`/`-q` to decrease verbose level.
//! - `--color` to set color mode
//!
//! The `prompt` feature enables these additional options:
//! - `--yes`/`-y` to answer `y` to all yes/no prompts.
//! - `--non-interactive`: Disallow prompts, prompts will fail with an error instead
//!   - With `--yes --non-interactive`, yes/no prompts gets answered `yes` and other prompts are
//!     blocked
//! - `--interactive`: This is the default, and cancels the effect of one `--non-interactive`
//!
//! The [`cu::cli`](macro@crate::cli) macro generates a shim
//! to parse the flags and pass it to your main function.
//! It also handles the `Result` returned back. See the example
//! below and more usage examples in the documentation for the macro.
//! ```rust,no_run
//! # use pistonite_cu as cu;
//! use cu::pre::*;
//! // clap will be part of the prelude
//! // when the `cli` feature is enabled
//!
//! // Typically, you want to have a wrapper struct
//! // so you can derive additional options with clap,
//! // and provide a description via doc comments, like below
//!
//! // clap will parse the doc comment of the Args struct
//! // as the help text
//!
//! /// My program
//! ///
//! /// This is my program, it is very good.
//! #[derive(clap::Parser, Clone)]
//! struct Args {
//!     /// Input of the program
//!     #[clap(short, long)]
//!     input: String,
//!     /// Output of the program
//!     #[clap(short, long)]
//!     output: Option<String>,
//!     #[clap(flatten)]
//!     inner: cu::cli::Flags,
//! }
//! // use the flags attribute to refer to the cu::cli::Flags field inside the Args struct
//! #[cu::cli(flags = "inner")]
//! fn main(args: Args) -> cu::Result<()> {
//!     cu::info!("input is {}", args.input);
//!     cu::info!("output is {:?}", args.output);
//!     Ok(())
//! }
//! ```
//!
//! # Printing and Logging
//! In addition to the logging macros re-exported from the [`log`](https://docs.rs/log)
//! crate, `cu` provides `print` and `hint` macros:
//! - `print`: like `info`, but has a higher importance
//! - `hint`: like `print`, but specifically for hinting actions the user can take
//!   (to resolve an error, for example).
//!
//! These 2 levels are not directly controlled by `log`,
//! and can still print when logging is statically disabled.
//!
//! The following table shows what are printed for each level,
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
//! # Other
//! When setting up test, you can use [`cu::cli::level`] to quickly inititialize logging
//! without dealing with the details.
//!
//! [`cu::cli::set_thread_name`] can be used to add a prefix to all messages printed
//! by the current thread.
//!
//! Messages that are too long and multi-line messages are automatically wrapped.
//!
//! # Manual Parsing CLI args
//! [`cu::cli::try_parse`](crate::cli::try_parse)
//! and [`cu::cli::print_help`](crate::cli::print_help) can be useful
//! when you want to manually invoke a command parser. These
//! respect the `--color` option passed to the program.
//!
//! # Progress Bars
//! See [Progress Bars](fn@crate::progress)
//!
//! # Prompting
//! See [Prompting](macro@crate::prompt)
//!
#[cfg(feature = "cli")]
mod flags;
#[cfg(all(feature = "coroutine", feature = "cli"))]
pub use flags::__co_run;
#[cfg(feature = "cli")]
pub use flags::{__run, Flags, print_help, try_parse};

mod print_init;
pub use print_init::level;
mod macros;
pub use macros::__print_with_level;

mod thread_name;
use thread_name::THREAD_NAME;
pub use thread_name::set_thread_name;
mod printer;

mod progress;
pub use progress::{ProgressBar, ProgressBarBuilder, progress};

#[cfg(feature = "prompt")]
mod prompt;
#[cfg(feature = "prompt")]
pub use prompt::{__prompt, __prompt_with_validation, __prompt_yesno};
#[cfg(feature = "prompt-password")]
mod password;
#[cfg(feature = "prompt-password")]
pub use password::password_chars_legal;

mod ctrlc;
pub use ctrlc::{catch_ctrlc, CtrlcSignal};
#[cfg(feature = "cli")]
pub use ctrlc::add_global_ctrlc_handler;
#[cfg(feature = "coroutine")]
pub use ctrlc::co_catch_ctrlc;

/// Formatting utils
pub(crate) mod fmt;

// 50ms between each cycle
const TICK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);
// 2B ticks * 10ms = 251 days.
// overflown tick means ETA will be inaccurate (after 251 days)
type Tick = u32;
