use std::path::PathBuf;
use std::ffi::OsStr;

use tokio::process::Command as TokioCommand;

use super::Config;

use crate::{co, pio, Context as _, PathExtension as _};

/// A [`Command`] to be built
pub type CommandBuilder = Command<(), (), ()>;

/// Builder for spawning a child process.
///
/// You can use the `command()` function on `Path` or `PathBuf`
/// to create a CommandBuilder.
///
/// ```rust,no_run
/// use std::path::Path;
///
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// let path = Path::new("ls");
/// path.command().wait_nz()?;
/// # Ok(()) }
/// ```
///
/// # Configuration
/// Configuring a `CommandBuilder` is much like configuring `Command`
/// from `std` or `tokio`. Notable features are:
/// - Use [`cu::args`](crate::args) and [`cu::envs`](crate::envs) macro
///   to enable adding multiple args or envs of different type in the same call.
///   Without the macro, `args`/`envs` only accept iterators that yield the same types.
/// - [`current_dir()`](Self::current_dir) will be normalized based on current process, before the child is spawned.
///   `std` recommends this, but not enforced. Here it's always done if set
/// - [`name()`](Self::name) sets an identifier for the process to be passed to IO handlers.
///   See below for more info about IO.
///
/// # Input-Output (IO)
/// IO is the most important part of dealing with child processes. After all, you would
/// want some information out of the process that you spawned.
///
/// Unlike `Command` in the standard library or `tokio`, `CommandBuilder` does not have
/// "default" IO behavior. You cannot spawn the child until you configured how you'd like
/// `stdout`, `stderr`, and `stdin` to behave.
///
pub struct Command<Out, Err, In> {
    /// Inner command
    command: TokioCommand,
    /// buffered current_dir to be set on the command before spawning
    current_dir: Option<PathBuf>,
    /// Name which maybe used by IO to print
    name: Option<String>,
    stdout: Out,
    stderr: Err,
    stdin: In,
}

// expose functions from tokio/std
#[rustfmt::skip]
impl<Out, Err, In> Command<Out, Err, In> {
    /// Add one argument
    ///
    /// Use [`Self::args`] or [`args`] macro to add more than one arguments
    /// in one call.
    #[inline(always)]
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self { self.command.arg(arg); self }

    /// Add multiple arguments
    ///
    /// This only accepts iterators of a single type, which forces
    /// you to call `.as_ref()` if the input has multiple types.
    /// To workaround this, use the [`args`] shorthand.
    #[inline(always)]
    pub fn args<I: IntoIterator<Item=S>, S:AsRef<OsStr>>(mut self, args: I) -> Self { self.command.args(args); self }


    /// Clear the environment variables, which will prevent
    /// inheriting environment variables from the parent (this process)
    #[inline(always)]
    pub fn env_clear(mut self) -> Self { self.command.env_clear(); self }

    /// Remove an environment variable
    #[inline(always)]
    pub fn env_remove(mut self, env: impl AsRef<OsStr>) -> Self { self.command.env_remove(env); self }

    /// Set a single environment variable
    #[inline(always)]
    pub fn env(mut self, k: impl AsRef<OsStr>, v: impl AsRef<OsStr>) -> Self { self.command.env(k, v); self }

    /// Set multiple environment variables
    ///
    /// This only accepts iterators of a single type, which forces
    /// you to call `.as_ref()` if the input has multiple types.
    /// To workaround this, use the [`envs`] macro shorthand.
    #[inline(always)]
    pub fn envs<I: IntoIterator<Item=(K,V)>,K:AsRef<OsStr>,V:AsRef<OsStr>>(mut self, envs: I) -> Self { self.command.envs(envs); self }


}

impl CommandBuilder {
    /// Start building a command. If the arg is a `Path` or `PathBuf`,
    /// you can also call `.command()` on it (remember to import prelude);
    pub fn new(bin: impl AsRef<OsStr>) -> Self {
        Self { 
            command: Command::new(bin),
            name: None,
            current_dir: None,
            stdout: (),
            stderr: (),
            stdin: ()
        }
    }
}

impl<Out, Err, In> CommandBuilder<Out, Err, In> {

    /// Add more configuration. See [`args!`](crate::args) and [`envs!`](crate::envs).
    #[inline(always)]
    pub fn add(mut self, config: impl Config) -> Self {
        config.configure(&mut self.command);
        self
    }

    /// Set the current working directory for the child process.
    ///
    /// Unlike `std`/`tokio` implementation, where canonicalizing the current
    /// dir is recommended, we always canonicalize the input here based on this
    /// process before spawning the child.
    pub fn current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    /// Configure child's standard output stream
    pub fn stdout<T: pio::ChildOutConfig>(self, config: T) -> CommandBuilder<T, Err, In> {
        CommandBuilder {
            command: self.command,
            current_dir: self.current_dir,
            name: self.name,
            stdout: config,
            stderr: self.stderr,
            stdin: self.stdin
        }
    }

    /// Configure child's standard error stream
    pub fn stderr<T: pio::ChildOutConfig>(self, config: T) -> CommandBuilder<Out, T, In> {
        CommandBuilder {
            command: self.command,
            current_dir: self.current_dir,
            name: self.name,
            stdout: self.stdout,
            stderr: config,
            stdin: self.stdin
        }
    }

    /// Configure child's both standard output and standard error with the same config
    pub fn stdboth<T: pio::ChildOutConfig + Clone>(self, config: T) -> CommandBuilder<T, T, In> {
        CommandBuilder {
            command: self.command,
            current_dir: self.current_dir,
            name: self.name,
            stdout: config.clone(),
            stderr: config,
            stdin: self.stdin
        }
    }

    /// Configure child's standard input stream
    pub fn stdin<T: pio::ChildInConfig>(self, config:T) -> CommandBuilder<Out, Err, T> {
        CommandBuilder {
            command: self.command,
            current_dir: self.current_dir,
            name: self.name,
            stdout: self.stdout,
            stderr: self.stderr,
            stdin: config
        }
    }

    /// Set the name of this command, which maybe used by output config
    /// to print in the terminal
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

// can only finish building once all IO are configured
impl<Out: pio::ChildOutConfig, Err: pio::ChildOutConfig, In: pio::ChildInConfig> CommandBuilder<Out, Err, In> {

    /// Spawn the child, and use the worker thread to monitor the child's IO.
    /// Returns a handle that can be used to wait for the child to be finished,
    /// or start to access the child's output on the current thread,
    /// as they come in
    pub fn spawn(mut self) -> crate::Result<pio::ConfiguredChild![Out, Err]> {
        use std::fmt::Write as _;
        let mut trace = String::new();
        let log_enabled = crate::log_enabled(crate::lv::T);
        if log_enabled {
            let command = self.command.as_std();
            let _ = write!(&mut trace, "spawning '{}', args: [", command.get_program().display());
            let mut args = command.get_args();
            if let Some(a) = args.next() {
                let arg = a.display().to_string().replace('\'', "\\'");
                let _ = write!(&mut trace, "'{arg}'");
            }
            for arg in args {
                let arg = arg.display().to_string().replace('\'', "\\'");
                let _ = write!(&mut trace, ", '{arg}'");
            }
            let _ = write!(&mut trace, "]");
        }
        if let Some(cd) = self.current_dir {
            let cd = cd.normalize_exists().with_context(|| {
                if log_enabled {
                    crate::trace!("error while {trace}");
                }
                crate::error!("cannot canonicalize current_dir: {}", cd.display());
                "cannot canonicalize current_dir while spawning child"
            })?;
            if log_enabled {
                let _ = write!(&mut trace, ", current_dir: '{}'", cd.display());
            }
            self.command.current_dir(cd);
        }
        if log_enabled {
            match &self.name {
                Some(name) => crate::debug!("[{name}] {trace}"),
                _ => crate::debug!("{trace}"),
            }
        }
        self.stdout.configure_stdout(&mut self.command);
        self.stderr.configure_stderr(&mut self.command);
        self.stdin.configure_stdin(&mut self.command).context("failed to configure child stdin")?;

        // self.command.spawn() must be called inside tokio
        let child = co::run(async move {
            let mut child = self.command.spawn().with_context(move || {
                crate::error!("failed to spawn command");
                "failed to spawn command"
            })?;
        
            let name = self.name.as_ref().map(|x| x.as_str());
        
            let stdout = self.stdout.take(&mut child, name, true).context("failed to take child stdout")?;
            let stderr = self.stderr.take(&mut child, name, false).context("failed to take child stderr")?;
            let stdin = self.stdin.take(&mut child).context("failed to take child stdin")?;
        
            use pio::ChildOutTask as _;
            use pio::ChildInTask as _;
            let (stdout_future, stdout) = stdout.run();
            let (stderr_future, stderr) = stderr.run();
            let stdin_future = stdin.run();
        
            // run the IO tasks in background thread
            let wait_task = co::spawn(async move {
                child.wait().await
            });
            let stdout_task = stdout_future.map(co::spawn);
            let stderr_task = stderr_future.map(co::spawn);
            let stdin_task = stdin_future.map(co::spawn);
        
            crate::Ok(super::child::LwChild {
                wait_task, stdout, stderr,
                stdin_task, stdout_task, stderr_task
            })
        });
        let child = child.join().context("failed to join the spawning task")??;
        Ok(child)
            
    }
}
