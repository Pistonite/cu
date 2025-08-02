//! CLI entry point. Integration with clap
use std::time::Instant;

use clap::{CommandFactory, FromArgMatches, Parser};

use crate::{ColorLevel, PrintLevel};

/// Test Message
///
/// Test longl ong long 
#[derive(Debug, Clone, PartialEq, Parser)]
#[clap(disable_help_flag(true))]
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

    /// Print help (--help for longer)
    #[clap(short = 'h', long)]
    help: bool,
}

impl AsRef<Flags> for Flags {
    fn as_ref(&self) -> &Flags {
        self
    }
}

impl Flags {
    /// Apply the CLI Flags
    ///
    /// This is unsafe because it modifies environment variables.
    /// The [`cu::cli`](crate::cli!) macro generates safe call to this
    /// when the program only has the main thread.
    pub unsafe fn apply(&self) {
        let level = self.verbose.clamp(0, 2) as i8 - self.quiet.clamp(0, 2) as i8;
        let level: PrintLevel = level.into();
        if level == PrintLevel::VerboseVerbose {
            if std::env::var("RUST_BACKTRACE")
                .unwrap_or_default()
                .is_empty() {
                unsafe {
                    std::env::set_var("RUST_BACKTRACE", "1")
                }
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
        crate::init_print_options(self.color.unwrap_or_default(), level.into(), prompt);
    }
}

/// Entry point
#[inline(always)]
pub unsafe fn run<T: clap::Parser + AsRef<Flags>, F: FnOnce(T) -> crate::Result<()>>(
    f: F,
) -> std::process::ExitCode {
    unsafe { __cli_run(f, |args| args.as_ref()) }
}

#[inline(always)]
#[doc(hidden)]
pub unsafe fn __cli_run<
T: clap::Parser,
F: FnOnce(T) -> crate::Result<()>,
FF: FnOnce(&T) -> &Flags
>(
    f: F, flags: FF
) -> std::process::ExitCode {
    let start = std::time::Instant::now();
    let args = unsafe { parse_args_or_help::<T, FF>(flags) };
    let result = f(args);
    handle_result(start, result)
}

/// Heavy-async entry point
#[inline(always)]
#[cfg(feature = "coroutine-heavy")]
pub unsafe fn run_heavy<
T: clap::Parser + AsRef<Flags>, 
F: FnOnce(T) -> X,
X: Future<Output=crate::Result<()>> + Send + 'static
>(
    f: F,
) -> std::process::ExitCode {
    let start = std::time::Instant::now();
    let args = <T as clap::Parser>::parse();
    let flags = args.as_ref();
    unsafe { flags.apply() };
    let result = crate::co::run_heavy(f(args));
    handle_result(start, result)
}

unsafe fn parse_args_or_help<T: Parser, F: FnOnce(&T) -> &Flags>(f: F) -> T {
    #[derive(Default, Clone, Parser)]
    struct OnlyColorFlag {
        #[clap(long)]
        color: Option<ColorLevel>,
    }
    let mut matches = <T as CommandFactory>::command().get_matches();
    let parsed = match <T as FromArgMatches>::from_arg_matches_mut(&mut matches) {
        Ok(x) => x,
        Err(e) => {
            // see if we can parse the color flag
            let color = OnlyColorFlag::try_parse().ok().unwrap_or_default().color.unwrap_or_default();
            let use_color = color.is_colored_for_stdout();
            eprintln!("{use_color}");
            print_help::<T>(use_color, false, Some(e))
        }
    };
    let flags = f(&parsed);
    if flags.help {
        let long = std::env::args().any(|x| x== "--help");
        let color = flags.color.unwrap_or_default();
        let use_color = color.is_colored_for_stdout();
        print_help::<T>(use_color, !long, None);
    }
    unsafe { flags.apply() };
    parsed
}

fn print_help<T: Parser>(color: bool, short: bool, error: Option<clap::Error>) -> ! {
    use clap::builder::styling::{Styles, AnsiColor, Effects, Style};

    let command = <T as CommandFactory>::command();
    let mut command = if color {
        
        //pub(crate) const NOP: Style = Style::new();
        pub(crate) const HEADER: Style = AnsiColor::Yellow.on_default().effects(Effects::BOLD);
        pub(crate) const USAGE: Style = AnsiColor::Red.on_default().effects(Effects::BOLD);
        pub(crate) const LITERAL: Style = AnsiColor::Cyan.on_default().effects(Effects::BOLD);
        pub(crate) const PLACEHOLDER: Style = AnsiColor::Cyan.on_default();
        pub(crate) const ERROR: Style = AnsiColor::Red.on_default().effects(Effects::BOLD);
        //pub(crate) const WARN: Style = AnsiColor::Yellow.on_default().effects(Effects::BOLD);
        //pub(crate) const NOTE: Style = AnsiColor::Cyan.on_default().effects(Effects::BOLD);
        //pub(crate) const GOOD: Style = AnsiColor::Green.on_default().effects(Effects::BOLD);
        pub(crate) const VALID: Style = AnsiColor::Cyan.on_default().effects(Effects::BOLD);
        pub(crate) const INVALID: Style = AnsiColor::Yellow.on_default().effects(Effects::BOLD);

        /// Cargo's color style
        /// [source](https://github.com/crate-ci/clap-cargo/blob/master/src/style.rs)
        pub(crate) const CARGO_STYLING: Styles = Styles::styled()
            .header(HEADER)
            .usage(USAGE)
            .literal(LITERAL)
            .placeholder(PLACEHOLDER)
            .error(ERROR)
            .valid(VALID)
            .invalid(INVALID)
        .context(AnsiColor::Yellow.on_default().effects(Effects::BOLD));

        let styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default())
    .usage(AnsiColor::Green.on_default())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Green.on_default());

        command.styles(CARGO_STYLING)
    } else {
        command.styles(Styles::plain())
    };
    match error {
        Some(error) => {
            let error = error.format(&mut command);
            error.exit()
        }
        None => {
            if short  {
                let _ = command.print_help();
            } else {
                let _ = command.print_long_help();
            }
            std::process::exit(0)
        }
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
