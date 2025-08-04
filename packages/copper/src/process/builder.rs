use std::ffi::OsStr;
use std::{path::PathBuf, process::ExitStatus};

use tokio::process::{Child as TokioChild, Command as TokioCommand};
use tokio::task::JoinSet;

use super::{Child, Config};

use crate::{Context as _, PathExtension as _, co, pio};

/// A [`Command`] to be built
pub type CommandBuilder = Command<(), (), ()>;

/// Builder for spawning a child process.
///
/// You can use the `command()` function on `Path` or `PathBuf`
/// to create a `CommandBuilder`.
///
/// ```rust,no_run
/// use std::path::Path;
///
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// let path = Path::new("ls");
/// path.command().all_null().wait_nz()?;
/// # Ok(()) }
/// ```
///
/// # Configuration
/// Configuring a `cu::Command` is much like configuring `Command`
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
/// See [`pio`] (Process IO) for configuration IO.
///
/// # Spawning
/// See [`spawn`](Self::spawn) and [`co_spawn`](Self::co_spawn).
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

impl CommandBuilder {
    /// Start building a command. If the arg is a `Path` or `PathBuf`,
    /// you can also call `.command()` on it (remember to import prelude)
    ///
    /// ```rust
    /// use cu::pre::*;
    ///
    /// # fn main() -> cu::Result<()> {
    /// let command = cu::CommandBuilder::new("git");
    /// // or:
    /// let command = std::path::Path::new("git").command();
    /// // even better, find the executable in PATH using the crate's
    /// // binary registry
    /// let command = cu::which("git")?.command();
    /// # Ok(()) }
    /// ```
    pub fn new(bin: impl AsRef<OsStr>) -> Self {
        Self {
            command: TokioCommand::new(bin),
            name: None,
            current_dir: None,
            stdout: (),
            stderr: (),
            stdin: (),
        }
    }
}

// expose functions from tokio/std
#[rustfmt::skip]
impl<Out, Err, In> Command<Out, Err, In> {
    /// Add one argument
    ///
    /// Use [`Self::args`] or [`args`](crate::args) macro to add more than one arguments
    /// in one call.
    #[inline(always)]
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self { self.command.arg(arg); self }

    /// Add multiple arguments
    ///
    /// This only accepts iterators of a single type, which forces
    /// you to call `.as_ref()` if the input has multiple types.
    /// To workaround this, use the [`args`](crate::args) shorthand.
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
    /// To workaround this, use the [`envs`](crate::envs) macro shorthand.
    #[inline(always)]
    pub fn envs<I: IntoIterator<Item=(K,V)>,K:AsRef<OsStr>,V:AsRef<OsStr>>(mut self, envs: I) -> Self { self.command.envs(envs); self }

    /// Add more configuration. See [`args!`](crate::args) and [`envs!`](crate::envs).
    #[inline(always)]
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, config: impl Config) -> Self {
        config.configure(&mut self.command);
        self
    }

    /// Set the current working directory for the child process.
    ///
    /// Unlike `std`/`tokio` implementation, where canonicalizing the current
    /// dir is recommended, we always canonicalize the input here based on this
    /// process before spawning the child.
    #[inline(always)]
    pub fn current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    /// Set the name of this command, which maybe used by output config
    /// to print in the terminal
    #[inline(always)]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Configure child's standard input stream
    #[inline(always)]
    pub fn stdin<T: pio::ChildInConfig>(self, config:T) -> Command<Out, Err, T> {
        Command {
            command: self.command,
            current_dir: self.current_dir,
            name: self.name,
            stdout: self.stdout,
            stderr: self.stderr,
            stdin: config,
        }
    }

    /// Configure child's standard input stream as [`pio::null()`]
    #[inline(always)]
    pub fn stdin_null(self) -> Command<Out, Err, pio::Null> {
        self.stdin(pio::null())
    }

    /// Configure child's standard input stream to inherit from parent ([`pio::inherit()`])
    #[inline(always)]
    pub fn stdin_inherit(self) -> Command<Out, Err, pio::Inherit> {
        self.stdin(pio::inherit())
    }

    /// Configure child's standard output stream
    #[inline(always)]
    pub fn stdout<T: pio::ChildOutConfig>(self, config: T) -> Command<T, Err, In> {
        Command {
            command: self.command,
            current_dir: self.current_dir,
            name: self.name,
            stdout: config,
            stderr: self.stderr,
            stdin: self.stdin
        }
    }

    /// Configure child's standard error stream
    #[inline(always)]
    pub fn stderr<T: pio::ChildOutConfig>(self, config: T) -> Command<Out, T, In> {
        Command {
            command: self.command,
            current_dir: self.current_dir,
            name: self.name,
            stdout: self.stdout,
            stderr: config,
            stdin: self.stdin
        }
    }

    /// Configure child's both standard output and standard error with the same config
    #[inline(always)]
    pub fn stdoe<T: pio::ChildOutConfig + Clone>(self, config: T) -> Command<T, T, In> {
        Command {
            command: self.command,
            current_dir: self.current_dir,
            name: self.name,
            stdout: config.clone(),
            stderr: config,
            stdin: self.stdin
        }
    }

    /// Configure child's standard output stream as [`pio::null()`]
    #[inline(always)]
    pub fn stdout_null(self) -> Command<pio::Null, Err, In> {
        self.stdout(pio::null())
    }

    /// Configure child's standard error stream as [`pio::null()`]
    #[inline(always)]
    pub fn stderr_null(self) -> Command<Out, pio::Null, In> {
        self.stderr(pio::null())
    }

    /// Configure both child's standard output and error streams as [`pio::null()`]
    #[inline(always)]
    pub fn stdoe_null(self) -> Command<pio::Null, pio::Null, In> {
        self.stdoe(pio::null())
    }

    /// Configure both child's standard input and error streams as [`pio::null()`]
    #[inline(always)]
    pub fn stdie_null(self) -> Command<Out, pio::Null, pio::Null> {
        self.stdin_null().stderr_null()
    }

    /// Configure both child's standard input and output streams as [`pio::null()`]
    #[inline(always)]
    pub fn stdio_null(self) -> Command<pio::Null, Err, pio::Null> {
        self.stdin_null().stdout_null()
    }

    /// Configure all standard input, output and error streams as [`pio::null()`]
    #[inline(always)]
    pub fn all_null(self) -> Command<pio::Null, pio::Null, pio::Null> {
        self.stdin_null().stdout_null().stderr_null()
    }

    /// Spawn the child and run the child's IO tasks in the background. **Note that
    /// you have to configure all 3 of `stdin`, `stdout` and `stderr`, before you can spawn the
    /// child. Otherwise, you will get a compile error that `spawn` is not defined**
    ///
    /// # Blocking
    /// Use [`co_spawn`](Self::co_spawn) if you are within an async context.
    ///
    /// # Return Value
    /// The return type shown above is a placeholder. The actual return
    /// value depends on the configuration for `stdout` and `stderr`,
    /// specifically, whether they have a handle to access the output.
    /// For example [`pio::inherit()`] or [`pio::spinner()`] don't
    /// have handles, because the output is directly printed,
    /// whereas [`pio::pipe()`] or [`pio::string()`] has handles for reading the output.
    ///
    /// - If both `stdout` and `stderr` don't have handle, then the output is [`Child`]
    /// ```rust
    /// use cu::pre::*; // The prelude import is required for `spawn`
    ///
    /// # fn main() -> cu::Result<()> {
    /// let child = cu::which("echo")?.command()
    ///    .arg("Hello, world!")
    ///    .stdout(cu::lv::I) // print stdout as info level log messages
    ///    .stdie_null()
    ///    .spawn()?;
    /// # Ok(()) }
    /// ```
    /// - Otherwise, the output is a 3-tuple ([`Child`], `Stdout`, `Stderr`),
    ///   where `Stdout` and `Stderr` are the respective handle type for the output configured.
    /// ```rust
    /// use cu::pre::*; // The prelude import is required for `spawn`
    ///
    /// # fn main() -> cu::Result<()> {
    /// let (child, lines, err) = cu::which("bash")?.command()
    ///    .args(["-c", r#"for i in {1..5}; do echo "Line $i"; sleep 1; done"# ])
    ///    .stdout(cu::pio::lines())
    ///    .stderr(cu::pio::string())
    ///    .stdin_null()
    ///    .spawn()?;
    /// # Ok(()) }
    /// ```
    /// - If `Stderr` doesn't have a handle but `Stdout` does, a 2-tuple
    ///   is also accepted.
    /// ```rust
    /// use cu::pre::*; // The prelude import is required for `spawn`
    ///
    /// # fn main() -> cu::Result<()> {
    /// let (child, lines) = cu::which("bash")?.command()
    /// // let (child, lines, _) also works
    ///    .args(["-c", r#"for i in {1..5}; do echo "Line $i"; sleep 1; done"# ])
    ///    .stdout(cu::pio::lines())
    ///    .stdie_null()
    ///    .spawn()?;
    /// # Ok(()) }
    /// ```
    #[cfg(doc)]
    pub fn spawn(self) -> crate::Result<EitherOf<Child, (Child, Out, Err)>> {
        panic!("this is a placeholder for documetnation, see below for implementation")
    }


    /// Spawn the child and run the child's IO tasks in the background. **Note that
    /// you have to configure all 3 of `stdin`, `stdout` and `stderr`, before you can spawn the
    /// child. Otherwise, you will get a compile error that `co_spawn` is not defined**
    ///
    /// Note that the child's IO are still drived by the background context.
    /// This is to prevent errors where blocking the current-thread runtime unexpectedly
    /// block the child's IO, even when the child's IO tasks are running on the background
    /// thread.
    ///
    /// # Return value
    /// See [`spawn`](Self::spawn)
    #[cfg(doc)]
    pub async fn co_spawn(self) -> crate::Result<EitherOf<Child, (Child, Out, Err)>> {
        panic!("this is a placeholder for documetnation, see below for implementation")
    }
}
#[cfg(doc)]
struct EitherOf<A, B>(A, B);

/// **Implementation only applies if the child has no output handles (see [`spawn`](Command::spawn) for more
/// details)**
impl<
    Out: pio::ChildOutConfig<__Null = pio::__OCNull>,
    Err: pio::ChildOutConfig<__Null = pio::__OCNull>,
    In: pio::ChildInConfig,
> Command<Out, Err, In>
{
    /// Spawn and wait for child to exit with non-zero status code.
    ///
    /// This is equivalent to calling [`spawn()`](Self::spawn), then
    /// [`wait_nz()`](Child::wait_nz) on the [`Child`].
    #[inline(always)]
    pub fn wait_nz(self) -> crate::Result<()> {
        self.spawn()?.wait_nz()
    }
    /// Spawn and wait for child to exit, returning its [`ExitStatus`] code
    ///
    /// This is equivalent to calling [`spawn()`](Self::spawn), then
    /// [`wait()`](Child::wait) on the [`Child`].
    #[inline(always)]
    pub fn wait(self) -> crate::Result<ExitStatus> {
        self.spawn()?.wait()
    }

    /// Spawn and wait for child to exit with non-zero status code, using
    /// the current tokio runtime context.
    ///
    /// This is equivalent to calling [`co_spawn()`](Self::co_spawn), then
    /// [`co_wait_nz()`](Child::co_wait_nz) on the [`Child`].
    #[inline(always)]
    pub async fn co_wait_nz(self) -> crate::Result<()> {
        self.co_spawn().await?.co_wait_nz().await
    }
    /// Spawn and wait for child to exit, returning its [`ExitStatus`] code,
    /// using the current tokio runtime context.
    ///
    /// This is equivalent to calling [`co_spawn()`](Self::co_spawn), then
    /// [`co_wait()`](Child::co_wait) on the [`Child`].
    #[inline(always)]
    pub async fn co_wait(self) -> crate::Result<ExitStatus> {
        self.co_spawn().await?.co_wait().await
    }
}

/// This trait allows implementing different return types for [`Command::spawn`] based on the configured IO.
pub trait Spawn<Target>
where
    Target: Send + 'static,
{
    fn spawn(self) -> crate::Result<Target>;
    fn co_spawn(self) -> crate::BoxedFuture<crate::Result<Target>>;
}

#[cfg(not(doc))]
macro_rules! Spawned {
    () => {
        $crate::Child
    };
    (Out) => {
        (
            $crate::Child,
            <Out::Task as $crate::process::pio::ChildOutTask>::Output,
        )
    };
    ($A:ident, $B:ident) => {
        (
            $crate::Child,
            <$A::Task as $crate::process::pio::ChildOutTask>::Output,
            <$B::Task as $crate::process::pio::ChildOutTask>::Output,
        )
    };
}

#[rustfmt::skip]
#[cfg(not(doc))]
impl< Out: pio::ChildOutConfig<__Null=pio::__OCNull>, Err: pio::ChildOutConfig<__Null=pio::__OCNull>, In: pio::ChildInConfig> Spawn<Spawned![]> for Command<Out, Err, In> {
    #[inline(always)]
    fn spawn(self) -> crate::Result<Spawned![]> {
        spawn_internal(self).map(|x| x.0)
    }
    #[inline(always)]
    fn co_spawn(self) -> crate::BoxedFuture<crate::Result<Spawned![]>> {
        Box::pin(async move {
            co_spawn_internal(self).await.map(|x| x.0)
        })
    }
}

#[rustfmt::skip]
#[cfg(not(doc))]
impl< Out: pio::ChildOutConfig<__Null=pio::__OCNonNull>, Err: pio::ChildOutConfig<__Null=pio::__OCNull>, In: pio::ChildInConfig> Spawn<Spawned![Out]> for Command<Out, Err, In> {
    #[inline(always)]
    fn spawn(self) -> crate::Result<Spawned![Out]> {
        spawn_internal(self).map(|(c,o,_)| (c,o))
    }
    #[inline(always)]
    fn co_spawn(self) -> crate::BoxedFuture<crate::Result<Spawned![Out]>> {
        Box::pin(async move {
            co_spawn_internal(self).await.map(|(c,o,_)| (c,o))
        })
    }
}

#[rustfmt::skip]
#[cfg(not(doc))]
impl< Out: pio::ChildOutConfig<__Null=pio::__OCNonNull>, Err: pio::ChildOutConfig, In: pio::ChildInConfig> Spawn<Spawned![Out, Err]> for Command<Out, Err, In> {
    #[inline(always)]
    fn spawn(self) -> crate::Result<Spawned![Out, Err]> {
        spawn_internal(self)
    }
    #[inline(always)]
    fn co_spawn(self) -> crate::BoxedFuture<crate::Result<Spawned![Out, Err]>> {
        Box::pin(co_spawn_internal(self))
    }
}

/// handle the actual spawning
#[allow(clippy::type_complexity)]
fn spawn_internal<Out: pio::ChildOutConfig, Err: pio::ChildOutConfig, In: pio::ChildInConfig>(
    mut self_: Command<Out, Err, In>,
) -> crate::Result<(
    Child,
    <Out::Task as pio::ChildOutTask>::Output,
    <Err::Task as pio::ChildOutTask>::Output,
)> {
    pre_spawn(&mut self_)?;

    // self.command.spawn() must be called on the background runtime,
    // because the IO will be attached to the active runtime context
    // if we call .spawn() on the current-thread runtime, then blocking
    // the current-thread runtime will also block the child's IO
    co::spawn(async move {
        let child = self_.command.spawn().context("failed to spawn command")?;
        post_spawn(self_, child)
    })
    .join()?
}
/// handle the actual spawning
#[allow(clippy::type_complexity)]
async fn co_spawn_internal<
    Out: pio::ChildOutConfig,
    Err: pio::ChildOutConfig,
    In: pio::ChildInConfig,
>(
    mut self_: Command<Out, Err, In>,
) -> crate::Result<(
    Child,
    <Out::Task as pio::ChildOutTask>::Output,
    <Err::Task as pio::ChildOutTask>::Output,
)> {
    pre_spawn(&mut self_)?;

    // self.command.spawn() must be called on the background runtime,
    // because the IO will be attached to the active runtime context
    // if we call .spawn() on the current-thread runtime, then blocking
    // the current-thread runtime will also block the child's IO
    co::spawn(async move {
        let child = self_.command.spawn().context("failed to spawn command")?;
        post_spawn(self_, child)
    })
    .co_join()
    .await?
}

fn pre_spawn<Out: pio::ChildOutConfig, Err: pio::ChildOutConfig, In: pio::ChildInConfig>(
    self_: &mut Command<Out, Err, In>,
) -> crate::Result<()> {
    use std::fmt::Write as _;
    let mut trace = String::new();

    // build the trace message
    let log_enabled = crate::log_enabled(crate::lv::T);
    if log_enabled {
        let command = self_.command.as_std();
        let _ = write!(
            &mut trace,
            "spawning '{}', args: [",
            command.get_program().display()
        );
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

    // configure final things on the Command
    if let Some(cd) = &self_.current_dir {
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
        self_.command.current_dir(cd);
    }

    if log_enabled {
        match &self_.name {
            Some(name) => crate::trace!("[{name}] {trace}"),
            _ => crate::trace!("{trace}"),
        }
    }
    // configure IO
    self_.stdout.configure_stdout(&mut self_.command);
    self_.stderr.configure_stderr(&mut self_.command);
    self_
        .stdin
        .configure_stdin(&mut self_.command)
        .context("failed to configure child stdin")?;
    Ok(())
}

#[allow(clippy::type_complexity)]
fn post_spawn<Out: pio::ChildOutConfig, Err: pio::ChildOutConfig, In: pio::ChildInConfig>(
    self_: Command<Out, Err, In>,
    mut child: TokioChild,
) -> crate::Result<(
    Child,
    <Out::Task as pio::ChildOutTask>::Output,
    <Err::Task as pio::ChildOutTask>::Output,
)> {
    let name = self_.name.as_deref();

    let stdout = self_
        .stdout
        .take(&mut child, name, true)
        .context("failed to take child stdout")?;
    let stderr = self_
        .stderr
        .take(&mut child, name, false)
        .context("failed to take child stderr")?;
    let stdin = self_
        .stdin
        .take(&mut child)
        .context("failed to take child stdin")?;

    // get the IO tasks
    use pio::ChildInTask as _;
    use pio::ChildOutTask as _;
    let (stdout_future, stdout) = stdout.run();
    let (stderr_future, stderr) = stderr.run();
    let stdin_future = stdin.run();

    let combined_future = async move {
        let mut j = JoinSet::new();
        if let Some(x) = stdin_future {
            j.spawn(x);
        }
        if let Some(x) = stdout_future {
            j.spawn(x);
        }
        if let Some(x) = stderr_future {
            j.spawn(x);
        }
        let mut panicked = false;
        while let Some(x) = j.join_next().await {
            // could not join because panicked
            if x.is_err() {
                panicked = true;
            }
        }
        panicked
    };
    let io_task = co::spawn(combined_future);

    Ok((
        Child {
            inner: child,
            io_task,
        },
        stdout,
        stderr,
    ))
}
