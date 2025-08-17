use std::sync::atomic::{AtomicBool, Ordering};

use crate::{Atomic, lv};

pub(crate) static PRINT_LEVEL: Atomic<u8, lv::Print> = Atomic::new_u8(lv::Print::Normal as u8);
pub(crate) static USE_COLOR: AtomicBool = AtomicBool::new(true);

/// Check if the logging level is enabled
pub fn log_enabled(lv: lv::Lv) -> bool {
    lv.can_print(PRINT_LEVEL.get())
}

/// Get if color printing is enabled
pub fn color_enabled() -> bool {
    USE_COLOR.load(Ordering::Acquire)
}

static ENABLE_TRACE_HINT: AtomicBool = AtomicBool::new(true);
static ENABLE_PRINT_TIME: AtomicBool = AtomicBool::new(true);

/// Disable printing the trace hint line if the CLI entry point returns an error
///
/// By default, the hint is displayed if `RUST_BACKTRACE` env var is not set
#[inline(always)]
pub fn disable_trace_hint() {
    ENABLE_TRACE_HINT.store(false, Ordering::Release);
}

/// Check if the "use -vv to display backtrace" will be printed on error
#[inline(always)]
pub fn is_trace_hint_enabled() -> bool {
    ENABLE_TRACE_HINT.load(Ordering::Acquire)
}

/// Disable printing the time took to run the command
#[inline(always)]
pub fn disable_print_time() {
    ENABLE_PRINT_TIME.store(false, Ordering::Release);
}

/// Check if the "finished in TIME" line will be printed on exit
#[inline(always)]
pub fn is_print_time_enabled() -> bool {
    ENABLE_PRINT_TIME.load(Ordering::Acquire)
}

/// Color Level settable with `--color` flag
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum Color {
    Always,
    Never,
    #[default]
    Auto,
}
impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Always => write!(f, "always"),
            Self::Never => write!(f, "never"),
            Self::Auto => write!(f, "auto"),
        }
    }
}
impl Color {
    /// Get if color should be used. If `Auto`, returns if stdout is terminal.
    pub fn is_colored_for_stdout(self) -> bool {
        use std::io::IsTerminal;
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => std::io::stdout().is_terminal(),
        }
    }

    /// Return the first `--color <COLOR>` or `--color=<COLOR>`
    /// found in os args
    pub fn from_os_args() -> Self {
        // for efficiency, we always return the first one
        let mut found_color = false;
        for x in std::env::args() {
            if found_color {
                if x == "always" {
                    return Self::Always;
                }
                if x == "never" {
                    return Self::Never;
                }
                if x == "auto" {
                    return Self::Auto;
                }
                found_color = false;
                continue;
            }
            if x == "--color" {
                found_color = true;
                continue;
            }
            if x == "--color=always" {
                return Self::Always;
            }
            if x == "--color=never" {
                return Self::Never;
            }
            if x == "--color=auto" {
                return Self::Auto;
            }
        }
        Self::Auto
    }
}

/// Print level settable with `-v` and `-q` flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Print {
    QuietQuiet,
    Quiet,
    Normal,
    Verbose,
    VerboseVerbose,
}
impl From<i8> for Print {
    fn from(value: i8) -> Self {
        match value {
            ..=-2 => Self::QuietQuiet,
            -1 => Self::Quiet,
            0 => Self::Normal,
            1 => Self::Verbose,
            2.. => Self::VerboseVerbose,
        }
    }
}
impl From<u8> for Print {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::QuietQuiet,
            1 => Self::Quiet,
            3 => Self::Verbose,
            4 => Self::VerboseVerbose,
            _ => Self::Normal,
        }
    }
}
impl From<Print> for u8 {
    fn from(value: Print) -> Self {
        value as Self
    }
}
impl From<Print> for log::LevelFilter {
    fn from(value: Print) -> Self {
        match value {
            Print::QuietQuiet => log::LevelFilter::Off,
            Print::Quiet => log::LevelFilter::Error,
            Print::Normal => log::LevelFilter::Info,
            Print::Verbose => log::LevelFilter::Debug,
            Print::VerboseVerbose => log::LevelFilter::Trace,
        }
    }
}

/// Prompt level set with `--yes`, `--interactive`, and `--non-interactive` flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Prompt {
    /// Show prompts interactively
    Interactive,
    /// Automatically answer "Yes" to all yes/no prompts, and `Auto` for regular prompts
    Yes,
    /// Do not allow prompts (non-interactive). Attempting to show prompt will error
    No,
}
impl From<u8> for Prompt {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Yes,
            2 => Self::No,
            _ => Self::Interactive,
        }
    }
}
impl From<Prompt> for u8 {
    fn from(value: Prompt) -> Self {
        value as Self
    }
}

/// Level of a message/print event.
///
/// Shortcuts available at `cu::lv`, e.g. `cu::lv::E` is `Error`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Lv {
    Error,
    Hint,
    Print,
    Warn,
    Info,
    Debug,
    Trace,

    Off,
}
impl Lv {
    /// Check if the current print level can print this message level
    pub fn can_print(self, level: Print) -> bool {
        match self {
            Lv::Off => false,
            Lv::Error | Lv::Hint | Lv::Print => level != Print::QuietQuiet,
            Lv::Warn | Lv::Info => level > Print::Quiet,
            Lv::Debug => level > Print::Normal,
            Lv::Trace => level == Print::VerboseVerbose,
        }
    }
}
impl From<log::Level> for Lv {
    fn from(value: log::Level) -> Self {
        match value {
            log::Level::Error => Self::Error,
            log::Level::Warn => Self::Warn,
            log::Level::Info => Self::Info,
            log::Level::Debug => Self::Debug,
            log::Level::Trace => Self::Trace,
        }
    }
}
impl From<u8> for Lv {
    fn from(value: u8) -> Self {
        match value {
            0 => Lv::Error,
            1 => Lv::Hint,
            2 => Lv::Print,
            3 => Lv::Warn,
            4 => Lv::Info,
            5 => Lv::Debug,
            6 => Lv::Trace,
            _ => Lv::Off,
        }
    }
}
impl From<Lv> for u8 {
    fn from(value: Lv) -> Self {
        value as u8
    }
}
/// Error
pub const E: Lv = Lv::Error;
/// Hint
pub const H: Lv = Lv::Hint;
/// Print
pub const P: Lv = Lv::Print;
/// Warn
pub const W: Lv = Lv::Warn;
/// Info
pub const I: Lv = Lv::Info;
/// Debug
pub const D: Lv = Lv::Debug;
/// Trace
pub const T: Lv = Lv::Trace;
