//! CLI entry point and integration with `clap`
//!
//! # Command Options
//! The [`Flags`] struct implement `clap::Args` to provide common
//! options that integrates with the rest of the crate:
//! - `--verbose`/`-v` to increase verbose level.
//! - `--quiet`/`-q` to decrease verbose level.
//! - `--color` to set color mode
//!
//! If your program has user interaction, the `prompt` feature enables these options:
//! - `--yes`/`-y` to answer `y` to all yes/no prompts.
//! - `--non-interactive`: Disallow prompts, prompts will fail with an error instead
//! - `--interactive`: This is the default, and cancels the effect of one `--non-interactive`
//!
//! The [`cu::cli`](macro@crate::cli) macro generates a shim
//! to parse the flags and pass it to your main function.
//! It also handles the `Result` returned back
//! ```rust,no_run
//! // Typically, you want to have a wrapper struct
//! // so you can derive additional options with clap,
//! // and provide a description via doc comments, like below
//! // clap::Parser is re-exported, so if you don't require
//! // additional functions from clap, you can avoid adding it
//! // to dependency
//!
//! /// My program
//! ///
//! /// This is my program, it is very good.
//! #[derive(cu::cli::Parser, Clone)]
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
//! #[cu::cli(flags = "inner")]
//! fn main(args: Args) -> cu::Result<()> {
//!     cu::info!("input is {}", args.input);
//!     cu::info!("output is {:?}", args.output);
//!     Ok(())
//! }
//! ```
//!
//! If your program is simple or you don't need extra
//! description in the --help message, you can also use `cu::cli::Flags`
//! directly in `main`:
//! ```rust,no_run
//! #[cu::cli]
//! fn main(args: cu::cli::Flags) -> cu::Result<()> {
//!     Ok(())
//! }
//! ```
//!
//!
//!
//!
//! # Async Entry Point
//! For async usage, see the [`coroutine`](crate::co) concept.
//!
use std::time::Instant;

use clap::{Command, CommandFactory, FromArgMatches};

pub use clap::Parser;

use crate::{ColorLevel, PrintLevel};

#[derive(Default, Debug, Clone, PartialEq, Parser)]
pub struct Flags {
    /// Verbose. More -v makes it more verbose (opposite of --quiet)
    #[clap(short = 'v', long, action(clap::ArgAction::Count))]
    verbose: u8,
    /// Quiet. More -q makes it more quiet (opposite of --verbose)
    #[clap(short = 'q', long, action(clap::ArgAction::Count))]
    quiet: u8,
    /// Set the color mode for this program. May affect subprocesses spawned.
    ///
    /// Fooo
    #[clap(long)]
    color: Option<ColorLevel>,
    /// Automatically answer 'yes' to all yes/no prompts
    #[cfg(feature = "prompt")]
    #[clap(short = 'y', long)]
    yes: bool,
    /// Make all prompts fail with an error. (Cancels with one --interactive)
    #[cfg(feature = "prompt")]
    #[clap(long, action(clap::ArgAction::Count))]
    non_interactive: u8,
    /// Allow interactivity. Cancels with one --non-interactive
    #[cfg(feature = "prompt")]
    #[clap(long, action(clap::ArgAction::Count))]
    interactive: u8,
}

impl AsRef<Flags> for Flags {
    fn as_ref(&self) -> &Flags {
        self
    }
}

impl Flags {
    /// Apply the CLI Flags
    ///
    /// # Safety
    /// This is unsafe because it modifies environment variables.
    /// The [`cu::cli`](macro@crate::cli) macro generates safe call to this
    /// when the program only has the main thread.
    pub unsafe fn apply(&self) {
        let level = self.verbose.clamp(0, 2) as i8 - self.quiet.clamp(0, 2) as i8;
        let level: PrintLevel = level.into();
        if level == PrintLevel::VerboseVerbose {
            if std::env::var("RUST_BACKTRACE")
                .unwrap_or_default()
                .is_empty()
            {
                unsafe { std::env::set_var("RUST_BACKTRACE", "1") }
            }
        }

        let prompt = {
            #[cfg(feature = "prompt")]
            match self.non_interactive.min(i8::MAX as u8) as i8
                - self.interactive.min(i8::MAX as u8) as i8
            {
                ..0 => {
                    if self.yes {
                        Some(crate::PromptLevel::Yes)
                    } else {
                        Some(crate::PromptLevel::Interactive)
                    }
                }
                0 => {
                    if self.yes {
                        Some(crate::PromptLevel::Yes)
                    } else {
                        None
                    }
                }
                _ => Some(crate::PromptLevel::No),
            }
            #[cfg(not(feature = "prompt"))]
            {
                None
            }
        };
        crate::init_print_options(self.color.unwrap_or_default(), level, prompt);
    }
}

/// Entry point to CLI
///
/// # Safety
/// A safe wrapper is generated by the [`cu::cli`](macro@crate::cli) macro.
/// See [module level documentation](self) for more.
#[inline(always)]
pub unsafe fn run<T: clap::Parser + AsRef<Flags>, F: FnOnce(T) -> crate::Result<()>>(
    f: F,
) -> std::process::ExitCode {
    unsafe { __run(f, |args| args.as_ref()) }
}

#[inline(always)]
#[doc(hidden)]
pub unsafe fn __run<
    T: clap::Parser,
    F: FnOnce(T) -> crate::Result<()>,
    FF: FnOnce(&T) -> &Flags,
>(
    f: F,
    flags: FF,
) -> std::process::ExitCode {
    let start = std::time::Instant::now();
    let args = unsafe { parse_args_or_help::<T, FF>(flags) };
    let result = f(args);
    handle_result(start, result)
}

/// Entry point to CLI
///
/// # Safety
/// A safe wrapper is generated by the [`cu::cli`](macro@crate::cli) macro.
/// See [module level documentation](self) for more.
#[inline(always)]
#[cfg(feature = "coroutine")]
#[doc(hidden)]
pub unsafe fn co_run<
    T: clap::Parser + AsRef<Flags>,
    F: FnOnce(T) -> X,
    X: Future<Output = crate::Result<()>> + Send + 'static,
>(
    f: F,
) -> std::process::ExitCode {
    unsafe { __co_run(f, |args| args.as_ref()) }
}

#[inline(always)]
#[cfg(feature = "coroutine")]
#[doc(hidden)]
pub unsafe fn __co_run<
    T: clap::Parser,
    F: FnOnce(T) -> X,
    X: Future<Output = crate::Result<()>> + Send + 'static,
    FF: FnOnce(&T) -> &Flags,
>(
    f: F,
    flags: FF,
) -> std::process::ExitCode {
    let start = std::time::Instant::now();
    let args = unsafe { parse_args_or_help::<T, FF>(flags) };
    let result = crate::co::run(f(args));

    handle_result(start, result)
}

unsafe fn parse_args_or_help<T: Parser, F: FnOnce(&T) -> &Flags>(f: F) -> T {
    let parsed = parse_args::<T>();
    let flags = f(&parsed);
    unsafe { flags.apply() };
    parsed
}

/// Wrapper for clap parse to respect the color flag when printing help or error
fn parse_args<T: Parser>() -> T {
    // parse the color arg first, so that we can respect it when printing help
    let color = ColorLevel::from_os_args();
    let use_color = color.is_colored_for_stdout();
    let mut matches = get_colored_command::<T>(use_color).get_matches();

    match <T as FromArgMatches>::from_arg_matches_mut(&mut matches) {
        Ok(x) => x,
        Err(e) => {
            let mut command = get_colored_command::<T>(use_color);
            let error = e.format(&mut command);
            error.exit()
        }
    }
}

fn get_colored_command<T: Parser>(color: bool) -> Command {
    use clap::builder::styling::{AnsiColor, Effects, Styles};
    let command = <T as CommandFactory>::command();
    if color {
        // Modified version of Cargo's color style
        // [source](https://github.com/crate-ci/clap-cargo/blob/master/src/style.rs)
        command.styles(
            Styles::styled()
                .header(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
                .usage(AnsiColor::Red.on_default().effects(Effects::BOLD))
                .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
                .placeholder(AnsiColor::Cyan.on_default())
                .error(AnsiColor::Red.on_default().effects(Effects::BOLD))
                .valid(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
                .invalid(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
                .context(AnsiColor::Yellow.on_default().effects(Effects::BOLD)),
        )
    } else {
        command.styles(Styles::plain())
    }
}

fn handle_result(start: Instant, result: crate::Result<()>) -> std::process::ExitCode {
    let elapsed = start.elapsed().as_secs_f32();
    if let Err(e) = result {
        crate::debug!("finished in {elapsed:.2}s");
        crate::error!("fatal: {e:?}");
        if std::env::var("RUST_BACKTRACE")
            .unwrap_or_default()
            .is_empty()
        {
            crate::hint!("use -vv or set RUST_BACKTRACE=1 to get backtrace for the error above.");
        }
        std::process::ExitCode::FAILURE
    } else {
        crate::info!("finished in {elapsed:.2}s");
        std::process::ExitCode::SUCCESS
    }
}
