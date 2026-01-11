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
//! When setting up test, you can use [`log_init`](crate::log_init) to quickly inititialize logging
//! without dealing with the details.
//!
//! [`set_thread_print_name`](crate::set_thread_print_name) can be used to add a prefix to all messages printed
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
//! [`progress_bar`](crate::progress_bar) and [`progress_bar_lowp`](crate::progress_bar_lowp) are used to create a bar.
//! The only difference is that `lowp` doesn't print a message when the progress
//! is done (as if the bar was never there). The bar takes a message to indicate
//! the current action, and each update call can accept a message to indicate
//! the current step. When `bar` is dropped, it will print a done message.
//!
//! ```rust,no_run
//! # use pistonite_cu as cu;
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
//! [`progress_unbounded`](crate::progress_unbounded) and [`progress_unbounded_lowp`](crate::progress_unbounded_lowp) are variants
//! that doesn't display the total steps. Use `()` as the step placeholder
//! when updating the bar.
//!
//! # Prompting
//! With the `prompt` feature enabled, you can
//! use [`prompt!`](crate::prompt) and [`yesno!`](crate::yesno) to show prompts.
//!
//! The prompts are thread-safe, meaning
//! You can call them from multiple threads, and they will be queued to prompt the user one after
//! the other. Prompts are always shown regardless of verbosity. But when stdout is redirected,
//! they will not render in terminal.
//!
//! # Async Entry Point
//! For async usage, see the [`coroutine`](crate::co) concept.
//!
//! # Manual Parsing
//! [`cu::cli::try_parse`](crate::cli::try_parse)
//! and [`cu::cli::print_help`](crate::cli::print_help) can be useful
//! when you want to manually invoke a command parser. These
//! respect the `--color` option passed to the program.
#[cfg(feature = "cli")]
mod flags;
#[cfg(feature = "cli")]
pub use flags::{Flags, try_parse, print_help, __run};
#[cfg(all(feature = "coroutine", feature = "cli"))]
pub use flags::__co_run;

mod print_init;
pub use print_init::level;
mod macros;
pub use macros::__print_with_level;

mod thread_name;
pub use thread_name::set_thread_name;
use thread_name::THREAD_NAME;
mod printer;

mod progress;
pub use progress::{progress, ProgressBar, ProgressBarBuilder};

#[cfg(feature = "prompt")]
mod prompt;
#[cfg(feature = "prompt")]
pub use prompt::{__prompt, __prompt_yesno};
#[cfg(feature = "prompt-password")]
mod password;
#[cfg(feature = "prompt-password")]
pub use password::password_chars_legal;

/// Formatting utils
pub(crate) mod fmt;

// 50ms between each cycle
const TICK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);
// 2B ticks * 10ms = 251 days.
// overflown tick means ETA will be inaccurate (after 251 days)
type Tick = u32;
