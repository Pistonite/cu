use tokio::process::Command as TokioCommand;

/// Add arguments to the command
#[doc(hidden)]
pub trait Config {
    fn configure(self, command: &mut TokioCommand);
}

#[doc(hidden)]
pub struct __ConfigFn<F>(pub F)
where
    F: FnOnce(&mut TokioCommand);
impl<F: FnOnce(&mut TokioCommand)> Config for __ConfigFn<F> {
    #[inline(always)]
    fn configure(self, command: &mut TokioCommand) {
        self.0(command)
    }
}

/// Create a config to add multiple args of different types when building
/// a subprocess.
///
/// See [`CommandBuilder`](crate::CommandBuilder) for more info on spawning
/// child processes.
///
/// # Example
/// ```rust,no_run
/// use std::path::Path;
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// let path = Path::new("foo");
/// cu::which("ls")?.command()
///    // without the macros, you can't mix `&Path` and `&str`
///    .add(cu::args![path, "-a"])
///    // ... more config
/// # ;
/// # Ok(()) }
/// ```
#[macro_export]
macro_rules! args {
    ($($arg:expr),* $(,)?) => {
        $crate::__priv::__ConfigFn(|c| {
            $( c.arg($arg); )*
        })
    };
}

/// Create a config to add multiple environments of different types when building
/// a subprocess.
///
/// # Example
/// ```rust,no_run
/// use std::path::Path;
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// let path = Path::new("bizbar");
/// cu::which("foo")?.command()
///    .add(cu::envs!{
///         "BAR" => "true",
///         "BIZ" => path
///    })
///    // ... more config
/// # ;
/// # Ok(()) }
/// ```
#[macro_export]
macro_rules! envs {
    ($($k:expr => $v:expr),* $(,)?) => {
        $crate::__priv::__ConfigFn(|c| {
            $( c.env($k, $v); )*
        })
    };
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ColorFlag(bool);
impl ColorFlag {
    #[inline(always)]
    pub fn use_eq_sign(self) -> bool {
        self.0
    }
}
impl std::fmt::Display for ColorFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let flag = if crate::color_enabled() {
            "always"
        } else {
            "never"
        };
        if self.use_eq_sign() {
            write!(f, "--color={flag}")
        } else {
            write!(f, "--color {flag}")
        }
    }
}
impl Config for ColorFlag {
    fn configure(self, command: &mut TokioCommand) {
        let flag = if crate::color_enabled() {
            "always"
        } else {
            "never"
        };
        if self.use_eq_sign() {
            command.arg(format!("--color={flag}"));
        } else {
            command.args(["--color", flag]);
        }
    }
}

/// Create a `--color always|never` flag that can be added to a command,
/// based on if color is enabled for the current process using
/// this crate's cli flags
///
/// # Example
/// ```rust,no_run
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// cu::which("git")?
///   .command()
///   .add(cu::color_flag());
///    // ... more config
/// # ;
/// # Ok(()) }
/// ```
#[inline(always)]
pub fn color_flag() -> ColorFlag {
    ColorFlag(false)
}

/// Create a `--color=always|never` flag that can be added to a command,
/// based on if color is enabled for the current process using
/// this crate's cli flags
///
/// # Example
/// ```rust,no_run
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// cu::which("git")?
///   .command()
///   .add(cu::color_flag_eq());
///    // ... more config
/// # ;
/// # Ok(()) }
/// ```
#[inline(always)]
pub fn color_flag_eq() -> ColorFlag {
    ColorFlag(true)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct WidthFlag(bool);
impl WidthFlag {
    #[inline(always)]
    pub fn use_eq_sign(self) -> bool {
        self.0
    }
}
impl Config for WidthFlag {
    fn configure(self, command: &mut TokioCommand) {
        if let Some(w) = crate::term_width() {
            if self.use_eq_sign() {
                command.arg(format!("--width={w}"));
            } else {
                command.arg("--width");
                command.arg(w.to_string());
            }
        }
    }
}

/// Create a `--width WIDTH` flag that can be added to a command,
/// based on the current width of the terminal
///
/// # Example
/// ```rust,no_run
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// cu::which("ls")?
///   .command()
///   .add(cu::width_flag());
///    // ... more config
/// # ;
/// # Ok(()) }
/// ```
#[inline(always)]
pub fn width_flag() -> WidthFlag {
    WidthFlag(false)
}

/// Create a `--width=WIDTH` flag that can be added to a command,
/// based on the current width of the terminal
///
/// # Example
/// ```rust,no_run
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// cu::which("ls")?
///   .command()
///   .add(cu::width_flag_eq());
///    // ... more config
/// # ;
/// # Ok(()) }
/// ```
#[inline(always)]
pub fn width_flag_eq() -> WidthFlag {
    WidthFlag(true)
}
