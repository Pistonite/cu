use std::path::PathBuf;
use std::ffi::OsStr;

use super::{Command, Config};

/// Builder for spawning a child process.
///
/// You can use the `command()` function on `Path` or `PathBuf`
/// to create a CommandBuilder.
///
/// ```rust,no_run
/// use cu::prelude::*;
///
/// let path = Path::new("ls");
/// let _ = path.command().wait()?;
/// ```
pub struct CommandBuilder {
    /// Inner command builder
    command: Command,
    /// buffered current_dir to be set on the command before spawning
    current_dir: Option<PathBuf>,
}

// expose functions from tokio/std
#[rustfmt::skip]
impl CommandBuilder {
    /// Add one argument
    ///
    /// Use [`Self::args`] or [`args`] macro to add more than one arguments
    /// in one call.
    #[inline(always)]
    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self { self.command.arg(arg); self }

    /// Add multiple arguments
    ///
    /// This only accepts iterators of a single type, which forces
    /// you to call `.as_ref()` if the input has multiple types.
    /// To workaround this, use the [`args`] shorthand.
    #[inline(always)]
    pub fn args<I: IntoIterator<Item=S>, S:AsRef<OsStr>>(&mut self, args: I) -> &mut Self { self.command.args(args); self }


    /// Clear the environment variables, which will prevent
    /// inheriting environment variables from the parent (this process)
    #[inline(always)]
    pub fn env_clear(&mut self) -> &mut Self { self.command.env_clear(); self }

    /// Remove an environment variable
    #[inline(always)]
    pub fn env_remove(&mut self, env: impl AsRef<OsStr>) -> &mut Self { self.command.env_remove(env); self }

    /// Set a single environment variable
    #[inline(always)]
    pub fn env(&mut self, k: impl AsRef<OsStr>, v: impl AsRef<OsStr>) -> &mut Self { self.command.env(k, v); self }

    /// Set multiple environment variables
    ///
    /// This only accepts iterators of a single type, which forces
    /// you to call `.as_ref()` if the input has multiple types.
    /// To workaround this, use the [`envs`] macro shorthand.
    #[inline(always)]
    pub fn envs<I: IntoIterator<Item=(K,V)>,K:AsRef<OsStr>,V:AsRef<OsStr>>(&mut self, envs: I) -> &mut Self { self.command.envs(envs); self }


}

impl CommandBuilder {
    /// Start building a command. If the arg is a `Path` or `PathBuf`,
    /// you can also call `.command()` on it (remember to import prelude);
    pub fn new(bin: impl AsRef<OsStr>) -> Self {
        Self { 
            command: Command::new(bin),
            current_dir: None
        }
    }

    /// Add more configuration. See [`args!`](crate::args) and [`envs!`](crate::envs).
    #[inline(always)]
    pub fn add(&mut self, config: impl Config) -> &mut Self {
        config.configure(&mut self.command);
        self
    }

    /// Set the current working directory for the child process.
    ///
    /// Unlike `std`/`tokio` implementation, where canonicalizing the current
    /// dir is recommended, we always canonicalize the input here based on this
    /// process before spawning the child.
    pub fn current_dir(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
        self.current_dir = Some(dir.into());
        self
    }
}
