use std::process::ExitStatus;

use tokio::task::JoinSet;
use tokio::process::Child as TokioChild;

use crate::{co,Context as _};

use super::pio;

// /// A Wrapper for spawned [`Child`] and its output handles
// ///
// /// ```rust
// /// use cu::pre::*;
// ///
// /// # #[cfg(unix)]
// /// # fn main() -> cu::Result<()> {
// /// let (child, out) = cu::which("echo")?.command()
// ///     .arg("Hello, world!")
// ///     .stdout(cu::pio::string())
// ///     .stdie_null()
// ///     .spawn()?;
// /// assert_eq!(out.join()?, "Hello, world!\n");
// /// child.wait_nz()?;
// /// # Ok(()) }
// ///
// /// ```
// pub struct Spawned<Out: pio::ChildOutTask, Err: pio::ChildOutTask> {
//     stdout: Out::Output,
//     stderr: Err::Output,
//     child: Child,
// }
// impl<Out: pio::ChildOutTask, Err: pio::ChildOutTask> Spawned<Out,Err> {
// }

/// A spawned child, where the IO are drived by the light-weight thread
pub struct Child {
    pub(crate) inner: TokioChild,
    pub(crate) io_task: co::Handle<bool>,
}

impl Child {
// impl<Out: pio::ChildOutTask, Err: pio::ChildOutTask> Child<Out,Err> {
    // /// Take the stdout handle, leaving a child with `()` in its place
    // #[inline(always)]
    // pub fn take_stdout(self) -> (Child<(), Err>, Out::Output) {
    //     (Child {
    //         inner: self.inner,
    //         stdin_task: self.stdin_task,
    //         stdout: (),
    //         stdout_task: self.stdout_task,
    //         stderr: self.stderr,
    //         stderr_task: self.stderr_task
    //     }, self.stdout)
    // }
    //
    // /// Mutably access the stderr handle of the child (usually to read result)
    // pub fn stderr_mut(&mut self) -> &mut Err::Output {
    //     &mut self.stderr
    // }

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
    pub fn wait(mut self) -> crate::Result<ExitStatus> {
        // consume the child by waiting
        let wait_task = co::spawn(async move {
            self.inner.wait().await
        });
        // ensure the IO tasks are finished first, since blocking
        // on child could dead lock if the child is waiting for IO
        let io_panicked = match self.io_task.join_maybe_aborted() {
            Ok(Some(panicked)) => panicked,
            Ok(None) => false, // aborted
            Err(_) => true
        };
        // let io_panicked = co::run(async move {
        //     let mut j = JoinSet::new();
        //     if let Some(x) = self.stdin_task {
        //         j.spawn(async move {
        //             x.co_join_maybe_aborted().await
        //         });
        //     }
        //     if let Some(x) = self.stdout_task {
        //         j.spawn(async move {
        //             x.co_join_maybe_aborted().await
        //         });
        //     }
        //     if let Some(x) = self.stderr_task {
        //         j.spawn(async move {
        //             x.co_join_maybe_aborted().await
        //         });
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
        if io_panicked {
            crate::warn!("some io tasks panicked while waiting for child process");
        }
        wait_task.join()?.context("io error while joining a child process")
    }
}

// pub struct Output<O, E> {
//     pub status: ExitStatus
//     pub stdout
// }
