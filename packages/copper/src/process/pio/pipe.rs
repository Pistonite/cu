use std::process::Stdio;

use tokio::process::{Command as TokioCommand, Child as TokioChild};

use crate::{BoxedFuture, Context as _};

use super::{ChildOutConfig, ChildInConfig, ChildOutTask};

/// Pipe the child's output into another command's stdin.
///
/// # Example
/// ```rust,no_run
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// let (hello, out) = cu::which("echo")?.command()
///     .arg("Hello, world!")
///     .stdout(cu::pio::pipe())
///     .stdie_null()
///     .spawn()?;
///
/// cu::which("rev")?.command()
///     .stdin(out)
///     .stdoe(cu::lv::I)
///     .wait_nz()?;
/// hello.wait_nz()?;
/// # Ok(()) }
/// ```
pub fn pipe() -> Pipe { Pipe }
pub struct Pipe;
impl ChildOutConfig for Pipe {
    type Task = PipeTask;
    type __Null = super::__OCNonNull;

    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(std::process::Stdio::piped());
    }

    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(std::process::Stdio::piped());
    }

    fn take(self, child: &mut TokioChild, _name: Option<&str>, is_out: bool) -> crate::Result<Self::Task> {
        let stream = super::take_child_out(child, is_out)?;
        // note: the falliable conversion is necessary right now
        // since tokio does not yet have a way for us to take
        // the pipe out before it's converted to tokio's ChildStdout/ChildStderr
        // see https://github.com/tokio-rs/tokio/pull/7249#issuecomment-3146995525
        // once Command::spawn_with is stabilized, we can use it
        let x: Result<Stdio, _> = match stream {
            Ok(s) => s.try_into(),
            Err(s) => s.try_into()
        };
        let x = x.context("failed to convert tokio pipe to std pipe")?;
        Ok(PipeTask(x))
    }
}
pub struct PipeTask(Stdio);
impl ChildOutTask for PipeTask {
    type Output = PipeOutput;
    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        (None, PipeOutput(Some(self.0)))
    }
}

/// The output of [`pipe`]. Can be piped into another command's [`stdin`](crate::Command::stdin).
pub struct PipeOutput(Option<Stdio>); // this uses an option right now to avoid dealing with unsafe
impl ChildInConfig for PipeOutput {
    type Task = ();

    fn configure_stdin(&mut self, command: &mut TokioCommand) -> crate::Result<()> {
        match self.0.take() {
            Some(x) => {
                command.stdin(x);
                Ok(())
            }
            _ => crate::bail!("unexpected: pipe was already taken")
        }
    }

    fn take(self, _: &mut TokioChild) -> crate::Result<Self::Task> {
        Ok(())
    }
}
