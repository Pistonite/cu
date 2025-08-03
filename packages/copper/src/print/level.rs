use clap::ValueEnum;

/// Color Level settable with `--color` flag
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
pub enum ColorLevel {
    Always,
    Never,
    #[default]
    Auto,
}
impl std::fmt::Display for ColorLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorLevel::Always => write!(f, "always"),
            ColorLevel::Never => write!(f, "never"),
            ColorLevel::Auto => write!(f, "auto"),
        }
    }
}
impl ColorLevel {
    /// Get if color should be used. If `Auto`, returns if stdout is terminal.
    pub fn is_colored_for_stdout(self) -> bool {
    use std::io::IsTerminal;
        match self {
            ColorLevel::Always => true,
            ColorLevel::Never => false,
            ColorLevel::Auto => std::io::stdout().is_terminal(),
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
                    return Self::Always
                }
                if x == "never" {
                    return Self::Never
                }
                if x == "auto" {
                    return Self::Auto
                }
                found_color = false;
                continue;
            }
            if x == "--color" {
                found_color = true;
                continue;
            }
            if x == "--color=always" {
                return Self::Always
            }
            if x == "--color=never" {
                return Self::Never
            }
            if x == "--color=auto" {
                return Self::Auto
            }
        }
        Self::Auto
    }
}


/// Print level settable with `-v` and `-q` flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PrintLevel {
    QuietQuiet,
    Quiet,
    Normal,
    Verbose,
    VerboseVerbose,
}
impl From<i8> for PrintLevel {
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
impl From<u8> for PrintLevel {
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
impl From<PrintLevel> for u8 {
    fn from(value: PrintLevel) -> Self {
        value as Self
    }
}
impl From<PrintLevel> for log::LevelFilter {
    fn from(value: PrintLevel) -> Self {
        match value {
            PrintLevel::QuietQuiet => log::LevelFilter::Off,
            PrintLevel::Quiet => log::LevelFilter::Error,
            PrintLevel::Normal => log::LevelFilter::Info,
            PrintLevel::Verbose => log::LevelFilter::Debug,
            PrintLevel::VerboseVerbose => log::LevelFilter::Trace,
        }
    }
}

/// Prompt level set with `--yes`, `--interactive`, and `--non-interactive` flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PromptLevel {
    /// Show prompts interactively
    Interactive,
    /// Automatically answer "Yes" to all yes/no prompts, and `Auto` for regular prompts
    Yes,
    /// Do not allow prompts (non-interactive). Attempting to show prompt will error
    No,
}
impl From<u8> for PromptLevel {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Yes,
            2 => Self::No,
            _ => Self::Interactive,
        }
    }
}
impl From<PromptLevel> for u8 {
    fn from(value: PromptLevel) -> Self {
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

    Off
}
impl Lv {
    /// Check if the current print level can print this message level
    pub fn can_print(self, level: PrintLevel) -> bool {
        match self {
            Lv::Off => false,
            Lv::Error | Lv::Hint | Lv::Print => level != PrintLevel::QuietQuiet,
            Lv::Warn | Lv::Info => level > PrintLevel::Quiet,
            Lv::Debug => level > PrintLevel::Normal,
            Lv::Trace => level == PrintLevel::VerboseVerbose,
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
            _ => Lv::Off
        }
    }
}
impl From<Lv> for u8 {
    fn from(value: Lv) -> Self {
        value as u8
    }
}
