
use std::{process::Stdio, sync::Arc};

use spin::mutex::SpinMutex;
use tokio::process::{ChildStderr, ChildStdout};

use crate::{print::Lv, BoxedFuture, ProgressBar, Atomic, Context as _};

use super::{ChildOutConfig, ChildInConfig, ChildOutTask, ChildInTask, Command, Child, Driver, DriverOutput};








#[derive(Default)]
pub struct PipeOut; // is_out

pub fn pipe() -> PipeOut { PipeOut }

impl ChildOutConfig for PipeOut {
    type Task = PipeTask;

    fn configure_stdout(&mut self, command: &mut Command) {
        command.stdout(std::process::Stdio::piped());
    }

    fn configure_stderr(&mut self, command: &mut Command) {
        command.stderr(std::process::Stdio::piped());
    }

    fn take(self, child: &mut Child, _name: Option<&str>, is_out: bool) -> crate::Result<Self::Task> {
        if is_out {
            let Some(stdout) = child.stdout.take() else {
                // stdout is set to piped above, so we should always get it
                crate::bail!("unexpected: failed to take stdout from child in pipe");
            };
            // note: the falliable conversion is necessary right now
            // since tokio does not yet have a way for us to take
            // the pipe out before it's converted to tokio's ChildStdout/ChildStderr
            // see https://github.com/tokio-rs/tokio/pull/7249#issuecomment-3146995525
            let x: Result<Stdio, _> = stdout.try_into();
            let x = x.context("failed to convert tokio pipe to std pipe")?;
            Ok(PipeTask(x))
        } else {
            let Some(stderr) = child.stderr.take() else {
                // stderr is set to piped above, so we should always get it
                crate::bail!("unexpected: failed to take stderr from child in pipe");
            };
            // note: the falliable conversion is necessary right now
            // since tokio does not yet have a way for us to take
            // the pipe out before it's converted to tokio's ChildStdout/ChildStderr
            // see https://github.com/tokio-rs/tokio/pull/7249#issuecomment-3146995525
            let x: Result<Stdio, _> = stderr.try_into();
            let x = x.context("failed to convert tokio pipe to std pipe")?;
            Ok(PipeTask(x))
        }
        
    }
}
pub struct PipeTask(Stdio);
impl ChildOutTask for PipeTask {
    type Output = Pipe;
    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        (None, Pipe(Some(self.0)))
    }
}
pub struct Pipe(Option<Stdio>);
impl Pipe {
    pub fn take(&mut self) -> crate::Result<impl ChildInConfig> {
        match self.0.take() {
            Some(x) => Ok(PipeIn(Some(x))),
            _ => {
                crate::bail!("fail to take pipe, maybe it was already taken?");
            }
        }
    }
}
pub struct PipeIn(Option<Stdio>);

impl ChildInConfig for PipeIn {
    type Task = ();

    fn configure_stdin(&mut self, command: &mut Command) -> crate::Result<()> {
        match self.0.take() {
            Some(x) => {
                command.stdin(x);
                Ok(())
            }
            _ => {
                crate::bail!("fail to take pipe, maybe it was already taken?");
            }
        }
    }

    fn take(self, child: &mut Child) -> crate::Result<Self::Task> {
        Ok(())
    }
}
