//! Utility to configure IO with child process.
//!
//! The `cu::pio::*` functions can be used as arguments
//! into the [`stdin`], [`stdout`] and [`stderr`] functions
//! of the [`Command`] builder to configure how you want to interact
//! with the child process's IO.
//!
//! An input config can only be passed into [`stdin`], whereas
//! an output config can be used both for [`stdout`] and [`stderr`].
//! [`null`] and [`inherit`] can be used both as input and output.
//!
//! # Available configurations
//! - [`cu::pio::null()`]: Turn off the stream, essentially [`Stdio::null()`].
//! - [`cu::pio::inherit()`]: Turn off the stream, essentially [`Stdio::inherit()`].
//!   - Note that the child outputs will mess with output from this crate.
//! - [`cu::lv::*`]: Read the child's output and print them as a log message of the level.
//! - [`cu::pio::spinner()`]: Show a progress bar for the child's output. Optionally also
//!   print non-progress outputs as regular messages.
//! - [`cu::pio::pipe()`]: Get a [`Pipe`] from the output, which can be passed as
//!   an input config to another command to pipe the output to its stdin.
//! - [`cu::pio::buffer()`]: Buffer the output as a `Vec<u8>`.
//! - [`cu::pio::string()`]: Buffer the output as a `String`.
//! - [`cu::pio::lines()`]: Get a reader that can read the output line-by-line.
//!   - See [`cu::pio::co_lines()`] for using an async reader.
//!
//!
//! [`cu::pio::null()`]: null
//! [`cu::pio::inherit()`]: inherit
//! [`cu::lv::*`]: crate::lv
//! [`cu::pio::spinner()`]: function@spinner
//! [`cu::pio::pipe()`]: function@pipe
//! [`cu::pio::buffer()`]: function@buffer
//! [`cu::pio::string()`]: function@string
//! [`cu::pio::lines()`]: function@lines
//! [`cu::pio::co_lines()`]: function@co_lines
//! [`Command`]: super::Command
//! [`stdin`]: super::Command::stdin
//! [`stdout`]: super::Command::stdout
//! [`stderr`]: super::Command::stderr
//!
//!
use std::process::Stdio;

use tokio::process::{Child as TokioChild, ChildStderr, ChildStdout, Command as TokioCommand};

use crate::BoxedFuture;

mod pipe;
#[cfg(feature = "print")]
mod print;
mod read;
#[cfg(feature = "print")]
mod spinner;

/// internal task types used in trait implementations
pub mod config {
    pub use super::pipe::Pipe;
    pub use super::read::Buffer;
    pub use super::read::BufferString as String;
    pub use super::read::CoLines;
    pub use super::read::Lines;
    #[cfg(feature = "print")]
    pub use super::spinner::Spinner;
}

/// internal task types used in trait implementations
pub mod task {
    pub use super::pipe::PipeTask as Pipe;
    #[cfg(feature = "print")]
    pub use super::print::PrintTask as Print;
    pub use super::read::BufferStringTask as String;
    pub use super::read::BufferTask as Buffer;
    pub use super::read::CoLinesTask as CoLines;
    pub use super::read::LinesTask as Lines;
    #[cfg(feature = "print")]
    pub use super::spinner::SpinnerTask as Spinner;
}

/// internal output types used in trait implementations
pub mod output {
    pub use super::pipe::PipeOutput as Pipe;
    pub use super::read::CoLinesOutput as CoLines;
    pub use super::read::LinesOutput as Lines;
}

// internal re-exports
pub use output::{CoLines, Lines, Pipe};
// factory re-exports
pub use pipe::pipe;
pub use read::{buffer, co_lines, lines, string};
#[cfg(feature = "print")]
pub use spinner::spinner;

#[cfg(feature = "print")]
mod print_driver;
#[cfg(feature = "print")]
use print_driver::*;

/// Configuration for process output to be used with
/// [`Command::stdout`] and [`Command::stderr`]
///
/// This is essentially a marker, that is used to create tasks
/// when spawning the child to drive the IO. See [`ChildOutTask`].
///
/// See [module documentation](self) for list of available configs
///
/// [`Command::stdout`]: crate::Command::stdout
/// [`Command::stderr`]: crate::Command::stderr
pub trait ChildOutConfig: Send + 'static {
    type Task: ChildOutTask;
    #[doc(hidden)]
    type __Null;
    /// Configure the standard output using this config, called before spawning
    fn configure_stdout(&mut self, command: &mut TokioCommand);
    /// Configure the standard error using this config, called before spawning
    fn configure_stderr(&mut self, command: &mut TokioCommand);

    // === once tokio exposes a way for us to take from StdChild, this could be
    // used to optimize pipes
    // /// Take the bits needed for this out config from the child, but operating on std
    // ///
    // /// If `Err` or `Ok(Some)` is returned, then `take()` will not be called,
    // /// and it will be safe to implement it as `!unreachable()`
    // fn take_std(self, _child: &mut std::process::Child, _is_out: bool) -> crate::Result<Option<Self::Task>> where Self: Sized {
    //     Ok(None)
    // }

    /// Take the bits needed for this out config from the child
    fn take(
        self,
        child: &mut TokioChild,
        name: Option<&str>,
        is_out: bool,
    ) -> crate::Result<Self::Task>;
}

/// Configuration for process input to be used with
/// [`Command::stdin`]
///
/// This is essentially a marker, that is used to create tasks
/// when spawning the child to drive the IO. See [`ChildInTask`]
///
/// See [module documentation](self) for list of available configs
///
/// [`Command::stdin`]: crate::Command::stdin
pub trait ChildInConfig: Send + 'static {
    type Task: ChildInTask;
    /// Configure the standard input using this config
    fn configure_stdin(&mut self, command: &mut TokioCommand) -> crate::Result<()>;
    /// Take the bits needed for this in config from the child
    fn take(self, child: &mut TokioChild) -> crate::Result<Self::Task>;
}

/// Task created by [`ChildOutConfig`]
pub trait ChildOutTask {
    type Output: Send + 'static;

    /// Run the task.
    ///
    /// The first return value is a future that will be spawned on the runtime
    /// and driven internally. The output is accessible by the user.
    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output);
}
/// Task created by [`ChildInConfig`]
pub trait ChildInTask {
    /// Run the task.
    ///
    /// Return a future that will be spawned internally to drive writing
    /// data to the child.
    fn run(self) -> Option<BoxedFuture<()>>;
}
impl ChildOutTask for () {
    type Output = ();
    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        (None, ())
    }
}
impl ChildInTask for () {
    fn run(self) -> Option<BoxedFuture<()>> {
        None
    }
}

#[doc(hidden)]
pub struct __OCNull;
#[doc(hidden)]
pub struct __OCNonNull;

#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct Inherit;
/// Inherit the parent's stdin, stdout, or stderr.
pub fn inherit() -> Inherit {
    Inherit
}
impl ChildOutConfig for Inherit {
    type Task = ();
    type __Null = __OCNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::inherit());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::inherit());
    }
    fn take(self, _: &mut TokioChild, _: Option<&str>, _: bool) -> crate::Result<()> {
        Ok(())
    }
}
impl ChildInConfig for Inherit {
    type Task = ();
    fn configure_stdin(&mut self, command: &mut TokioCommand) -> crate::Result<()> {
        command.stdin(Stdio::inherit());
        Ok(())
    }
    fn take(self, _: &mut TokioChild) -> crate::Result<()> {
        Ok(())
    }
}
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct Null;
/// Direct the stream to null (i.e. ignore it)
pub fn null() -> Null {
    Null
}
impl ChildOutConfig for Null {
    type Task = ();
    type __Null = __OCNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::null());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::null());
    }
    fn take(self, _: &mut TokioChild, _: Option<&str>, _: bool) -> crate::Result<()> {
        Ok(())
    }
}
impl ChildInConfig for Null {
    type Task = ();
    fn configure_stdin(&mut self, command: &mut TokioCommand) -> crate::Result<()> {
        command.stdin(Stdio::null());
        Ok(())
    }
    fn take(self, _: &mut TokioChild) -> crate::Result<()> {
        Ok(())
    }
}

pub(crate) fn take_child_out(
    child: &mut TokioChild,
    is_out: bool,
) -> crate::Result<Result<ChildStdout, ChildStderr>> {
    if is_out {
        let Some(stdout) = child.stdout.take() else {
            crate::bail!("unexpected: failed to take stdout from child");
        };
        Ok(Ok(stdout))
    } else {
        let Some(stderr) = child.stderr.take() else {
            crate::bail!("unexpected: failed to take stderr from child");
        };
        Ok(Err(stderr))
    }
}
