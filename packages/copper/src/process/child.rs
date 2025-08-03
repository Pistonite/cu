use std::process::ExitStatus;

use tokio::task::JoinSet;

use crate::{co,Context as _};

use super::pio;

/// A spawned child, where the IO are drived by the light-weight thread
pub struct LwChild<Out: pio::ChildOutTask, Err: pio::ChildOutTask> {
    pub(crate) wait_task: co::Handle<std::io::Result<ExitStatus>>,
    pub(crate) stdin_task: Option<co::Handle<()>>,
    pub(crate) stdout: Out::Output,
    pub(crate) stdout_task: Option<co::Handle<()>>,
    pub(crate) stderr: Err::Output,
    pub(crate) stderr_task: Option<co::Handle<()>>,
}

impl <O: pio::ChildOutTask, E: pio::ChildOutTask> 
LwChild<O,E> {
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
        // ensure the IO tasks are finished first, since blocking
        // on child could dead lock if the child is waiting for IO
        todo!()
        // let io_panicked = co::run(async move {
        //     let mut j = JoinSet::new();
        //     if let Some(x) = self.stdin_task {
        //         j.spawn(x);
        //     }
        //     if let Some(x) = self.stdout_task {
        //         j.spawn(x);
        //     }
        //     if let Some(x) = self.stderr_task {
        //         j.spawn(x);
        //     }
        //     let mut panicked = false;
        //     while let Some(x) = j.join_next().await {
        //         let Ok(x) = x else {
        //             // could not join because panicked
        //             panicked = true;
        //             continue;
        //         };
        //         let Ok(_) = x else {
        //             // could not join because panicked
        //             panicked = true;
        //             continue;
        //         };
        //     }
        //     panicked
        // });
        // if io_panicked {
        //     crate::warn!("some io tasks panicked while waiting for child process");
        // }
        // crate::trace!("wait task");
        // let exit_result = co::run(self.wait_task)?;
        // // let exit_result = self.wait_task.join()?;
        // exit_result.context("io error while joining a child process")
    }
}

// pub struct Output<O, E> {
//     pub status: ExitStatus
//     pub stdout
// }
