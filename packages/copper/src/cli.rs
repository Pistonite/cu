//! CLI entry point and integration with `clap`
//!
//! When `cli` feature is enabled, `clap` is re-exported from the prelude,
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
//! If your program has user interaction, the `prompt` feature enables these options:
//! - `--yes`/`-y` to answer `y` to all yes/no prompts.
//! - `--non-interactive`: Disallow prompts, prompts will fail with an error instead
//! - `--interactive`: This is the default, and cancels the effect of one `--non-interactive`
//!
//! The [`cu::cli`](macro@crate::cli) macro generates a shim
//! to parse the flags and pass it to your main function.
//! It also handles the `Result` returned back
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
//! // The 'flags' attribute lets the generated code access the common flags
//! // in the cli struct. When omitted, the struct should implement AsRef<cu::cli::Flags>
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
//! # use pistonite_cu as cu;
//! #[cu::cli]
//! fn main(args: cu::cli::Flags) -> cu::Result<()> {
//!     Ok(())
//! }
//! ```
//!
//! Optionally, a preprocessor function can be provided to modify the structs
//! (typically the common flags) before applying them
//! ```rust,no_run
//! # use pistonite_cu as cu;
//! use cu::pre::*;
//!
//! #[derive(clap::Parser)]
//! struct Args {
//!     #[clap(subcommand)]
//!     subcommand: Option<Command>,
//!     #[clap(flatten)]
//!     inner: cu::cli::Flags,
//! }
//! impl Args {
//!     fn preprocess(&mut self) {
//!         // merge subcommand flags into top level flags
//!         if let Some(Command::Foo(c)) = &self.subcommand {
//!             self.inner.merge(c);
//!         }
//!     }
//! }
//! impl AsRef<cu::cli::Flags> for Args {
//!     fn as_ref(&self) -> &cu::cli::Flags {
//!         &self.inner
//!     }
//! }
//! #[derive(clap::Subcommand)]
//! enum Command {
//!     Foo(cu::cli::Flags),
//! }
//! #[cu::cli(preprocess = Args::preprocess)]
//! fn main(args: Args) -> cu::Result<()> {
//!     Ok(())
//! }
//! ```
//!
//! # Printing and Logging
//! By default, even without the `cli` feature, `cu` re-exports
//! the features from `log` so you can add logging and error handling (through
//! `anyhow`) by depending on `cu` from a library.
//!
//! For crates only used in binary, but is not a binary target (i.e.
//! some shared module used for binary targets), you can enable
//! the `print` feature to get access to the `print` and `hint` macros:
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
//!
use std::ffi::OsString;
use std::time::Instant;

use clap::{Command, CommandFactory, FromArgMatches, Parser};

use crate::lv;

#[derive(Default, Debug, Clone, PartialEq, Parser)]
pub struct Flags {
    /// Verbose. More -v makes it more verbose (opposite of --quiet)
    #[clap(short = 'v', long, action(clap::ArgAction::Count))]
    pub verbose: u8,
    /// Quiet. More -q makes it more quiet (opposite of --verbose)
    #[clap(short = 'q', long, action(clap::ArgAction::Count))]
    pub quiet: u8,
    /// Set the color mode for this program. May affect subprocesses spawned.
    #[clap(long)]
    pub color: Option<lv::Color>,
    /// Automatically answer 'yes' to all yes/no prompts
    #[cfg(feature = "prompt")]
    #[clap(short = 'y', long)]
    pub yes: bool,
    /// Make all prompts fail with an error. (Cancels with one --interactive)
    #[cfg(feature = "prompt")]
    #[clap(long, action(clap::ArgAction::Count))]
    pub non_interactive: u8,
    /// Allow interactivity. Cancels with one --non-interactive
    #[cfg(feature = "prompt")]
    #[clap(long, action(clap::ArgAction::Count))]
    pub interactive: u8,
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
        let level = (self.verbose as i8 - self.quiet as i8).clamp(-2, 2);
        let level: lv::Print = level.into();
        if level == lv::Print::VerboseVerbose {
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
                ..=0 => {
                    if self.yes {
                        Some(lv::Prompt::YesOrInteractive)
                    } else {
                        Some(lv::Prompt::Interactive)
                    }
                }
                _ => {
                    if self.yes {
                        Some(lv::Prompt::YesOrBlock)
                    } else {
                        Some(lv::Prompt::Block)
                    }
                }
            }
            #[cfg(not(feature = "prompt"))]
            {
                None
            }
        };
        crate::init_print_options(self.color.unwrap_or_default(), level, prompt);
    }

    /// Merge `other` into self. Options in other will be applied on top of self (equivalent
    /// to specifying `self` then specify `other`
    pub fn merge(&mut self, other: &Self) {
        self.verbose += other.verbose;
        self.quiet += other.quiet;
        if let Some(color) = other.color {
            self.color = Some(color);
        }
        #[cfg(feature = "prompt")]
        {
            if other.yes {
                self.yes = true;
            }
            self.non_interactive += other.non_interactive;
            self.interactive += other.interactive;
        }
    }
}

/// Entry point to CLI
///
/// # Safety
/// A safe wrapper is generated by the [`cu::cli`](macro@crate::cli) macro.
/// See [module level documentation](self) for more.
#[inline(always)]
#[doc(hidden)]
pub unsafe fn __run<
    TArg: clap::Parser,
    FPreproc: FnOnce(&mut TArg),
    FExecute: FnOnce(TArg) -> crate::Result<()>,
    FFlag: FnOnce(&TArg) -> &Flags,
>(
    fn_preproc: FPreproc,
    fn_execute: FExecute,
    fn_flag: FFlag,
) -> std::process::ExitCode {
    let start = std::time::Instant::now();
    let args = unsafe { parse_args_or_help::<TArg, FPreproc, FFlag>(fn_preproc, fn_flag) };
    let result = fn_execute(args);
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
pub unsafe fn __co_run<
    TArg: clap::Parser + Send + 'static,
    FPreproc: FnOnce(&mut TArg),
    FExecute: FnOnce(TArg) -> TResult + Send + 'static,
    TResult: Future<Output = crate::Result<()>> + Send + 'static,
    FFlag: FnOnce(&TArg) -> &Flags,
>(
    fn_preproc: FPreproc,
    fn_execute: FExecute,
    fn_flag: FFlag,
) -> std::process::ExitCode {
    let start = std::time::Instant::now();
    let args = unsafe { parse_args_or_help::<TArg, FPreproc, FFlag>(fn_preproc, fn_flag) };
    let result = crate::co::run(async move { fn_execute(args).await });

    handle_result(start, result)
}

unsafe fn parse_args_or_help<
    TArg: Parser,
    FPreproc: FnOnce(&mut TArg),
    FFlag: FnOnce(&TArg) -> &Flags,
>(
    fn_preproc: FPreproc,
    fn_flag: FFlag,
) -> TArg {
    let mut parsed = parse_args::<TArg>();
    fn_preproc(&mut parsed);
    let flags = fn_flag(&parsed);
    unsafe { flags.apply() };
    parsed
}

/// Wrapper for clap parse to respect the color flag when printing help or error
fn parse_args<T: Parser>() -> T {
    // parse the color arg first, so that we can respect it when printing help
    let color = lv::Color::from_os_args();
    let use_color = color.is_colored_for_stdout();

    // this will exit on error
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

/// Try to parse arguments from an iterator and print the error/help
/// on failure.
///
/// Whether the output has color depends on the main CLI `--color` option.
/// This is useful for implementing custom command parser within
/// an application.
pub fn try_parse<T: Parser, I: IntoIterator>(iter: I) -> Option<T>
where
    I::Item: Into<OsString> + Clone,
{
    let use_color = crate::color_enabled();
    let result = get_colored_command::<T>(use_color)
        .try_get_matches_from(iter)
        .and_then(|mut matches| <T as FromArgMatches>::from_arg_matches_mut(&mut matches));
    match result {
        Ok(x) => Some(x),
        Err(e) => {
            let mut command = get_colored_command::<T>(use_color);
            let error = e.format(&mut command);
            if let Err(e) = error.print() {
                crate::warn!("arg parse error failed to print: {e:?}");
            }
            None
        }
    }
}

/// Print the help text from a command parser.
///
/// Whether the output has color depends on the main CLI `--color` option.
/// This is useful for implementing custom command parser within
/// an application.
#[inline(always)]
pub fn print_help<T: Parser>(long: bool) {
    let command = get_colored_command::<T>(crate::color_enabled());
    print_help_impl(command, long)
}
fn print_help_impl(mut command: Command, long: bool) {
    let result = if long {
        command.print_long_help()
    } else {
        command.print_help()
    };
    if let Err(e) = result {
        crate::warn!("help failed to print: {e:?}");
    }
}

#[inline(always)]
fn get_colored_command<T: Parser>(color: bool) -> Command {
    get_colored_command_impl(<T as CommandFactory>::command(), color)
}
fn get_colored_command_impl(command: Command, color: bool) -> Command {
    use clap::builder::styling::{AnsiColor, Styles};
    if color {
        // Modified version of Cargo's color style
        // [source](https://github.com/crate-ci/clap-cargo/blob/master/src/style.rs)
        command.styles(
            Styles::styled()
                .header(AnsiColor::BrightYellow.on_default())
                .usage(AnsiColor::BrightRed.on_default())
                .literal(AnsiColor::BrightCyan.on_default())
                .placeholder(AnsiColor::Cyan.on_default())
                .error(AnsiColor::BrightRed.on_default())
                .valid(AnsiColor::BrightCyan.on_default())
                .invalid(AnsiColor::BrightYellow.on_default())
                .context(AnsiColor::BrightYellow.on_default()),
        )
    } else {
        command.styles(Styles::plain())
    }
}

fn handle_result(start: Instant, result: crate::Result<()>) -> std::process::ExitCode {
    let elapsed = start.elapsed().as_secs_f32();
    if let Err(e) = result {
        crate::error!("fatal: {e:?}");
        if crate::lv::is_trace_hint_enabled() {
            if std::env::var("RUST_BACKTRACE")
                .unwrap_or_default()
                .is_empty()
            {
                crate::hint!("use -vv or set RUST_BACKTRACE=1 to display the error backtrace.");
            }
        }
        if crate::lv::is_print_time_enabled() {
            crate::info!("finished in {elapsed:.2}s");
        }
        std::process::ExitCode::FAILURE
    } else {
        if crate::lv::is_print_time_enabled() {
            crate::info!("finished in {elapsed:.2}s");
        }
        std::process::ExitCode::SUCCESS
    }
}
