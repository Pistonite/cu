use std::process::ExitStatus;

use crate::{co,Context as _};

use super::pio;

/// A spawned child, where the IO are drived by the light-weight thread
pub struct LwChild<I: pio::ChildTask, O: pio::ChildTask, E: pio::ChildTask> {
    pub(crate) wait_task: co::LwHandle<std::io::Result<ExitStatus>>,
    pub(crate) stdin: I::Output,
    pub(crate) stdin_task: Option<co::LwHandle<()>>,
    pub(crate) stdout: O::Output,
    pub(crate) stdout_task: Option<co::LwHandle<()>>,
    pub(crate) stderr: E::Output,
    pub(crate) stderr_task: Option<co::LwHandle<()>>,
}

impl <I: pio::ChildTask, O: pio::ChildTask, E: pio::ChildTask> 
LwChild<I,O,E> {
    /// Access the stdin handle of the child
    pub fn stdin(&self) -> &I::Output {
        &self.stdin
    }

    /// Mutably access the stdin handle of the child
    pub fn stdin_mut(&mut self) -> &mut I::Output {
        &mut self.stdin
    }

    /// Mutably access the stdout handle of the child (usually to read result)
    pub fn stdout_mut(&mut self) -> &mut O::Output {
        &mut self.stdout
    }

    /// Mutably access the stderr handle of the child (usually to read result)
    pub fn stderr_mut(&mut self) -> &mut E::Output {
        &mut self.stderr
    }

    /// Block the thread and wait for the child to finish, and check if the ExitStatus is 0
    pub fn wait_nz(self) -> crate::Result<()> {
        let status = self.wait()?;
        if !status.success() {
            crate::bail!("child exited with non-zero status");
        }
        Ok(())
    }

    /// Block the thread and wait for the child to finish,
    /// and return the exit status only. Output handles are discarded
    pub fn wait(self) -> crate::Result<ExitStatus> {
        drop(self.stdin);
        // ensure the IO tasks are finished first, since blocking
        // on child could dead lock if the child is waiting for IO
        let mut handles = Vec::with_capacity(3);
        if let Some(x) = self.stdin_task {
            handles.push(x);
        }
        if let Some(x) = self.stdout_task {
            handles.push(x);
        }
        if let Some(x) = self.stderr_task {
            handles.push(x);
        }
        co::join(handles);
        let exit_result = self.wait_task.join()?;
        exit_result.context("io error while joining a child process")
    }
}

// pub struct Output<O, E> {
//     pub status: ExitStatus
//     pub stdout
// }
