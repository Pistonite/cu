use std::process::ExitStatus;

use tokio::process::Child as TokioChild;

use crate::{Context as _, co};

/// A child process spawned with [`Command`](crate::Command)
pub struct Child {
    pub(crate) inner: TokioChild,
    pub(crate) io_task: co::Handle<bool>,
}

impl Child {
    /// Block the thread and wait for the child to finish, and check if the ExitStatus is 0
    ///
    /// # Blocking
    /// This will block the current thread while trying to join the child.
    /// Use [`co_wait_nz`](Self::co_wait_nz) to avoid blocking if in async context.
    pub fn wait_nz(self) -> crate::Result<()> {
        let status = self.wait()?;
        if !status.success() {
            crate::bail!("child exited with non-zero status");
        }
        Ok(())
    }

    /// Block the thread and wait for the child to finish,
    /// and return its [`ExitStatus`]
    ///
    /// # Blocking
    /// This will block the current thread while trying to join the child.
    /// Use [`co_wait`](Self::co_wait) to avoid blocking if in async context.
    pub fn wait(mut self) -> crate::Result<ExitStatus> {
        // consume the child by waiting
        let wait_task = co::spawn(async move { self.inner.wait().await });
        // ensure the IO tasks are finished first, since blocking
        // on child could dead lock if the child is waiting for IO
        let io_panicked = match self.io_task.join_maybe_aborted() {
            Ok(Some(panicked)) => panicked,
            Ok(None) => false, // aborted
            Err(_) => true,
        };
        if io_panicked {
            crate::warn!("some io tasks panicked while waiting for child process");
        }
        wait_task
            .join()?
            .context("io error while joining a child process")
    }

    /// Wait for the child asynchronously using the current tokio runtime,
    /// and check if the ExitStatus is 0.
    ///
    /// # Panic
    /// Will panic if called outside of a tokio runtime context
    pub async fn co_wait_nz(self) -> crate::Result<()> {
        let status = self.co_wait().await?;
        if !status.success() {
            crate::bail!("child exited with non-zero status");
        }
        Ok(())
    }

    /// Wait for the child asynchronously using the current tokio runtime,
    /// and return its [`ExitStatus`]
    ///
    /// # Panic
    /// Will panic if called outside of a tokio runtime context
    pub async fn co_wait(mut self) -> crate::Result<ExitStatus> {
        // consume the child by waiting
        let wait_task = co::co_spawn(async move { self.inner.wait().await });
        // ensure the IO tasks are finished first, since blocking
        // on child could dead lock if the child is waiting for IO
        let io_panicked = match self.io_task.co_join_maybe_aborted().await {
            Ok(Some(panicked)) => panicked,
            Ok(None) => false, // aborted
            Err(_) => true,
        };
        if io_panicked {
            crate::warn!("some io tasks panicked while waiting for child process");
        }
        wait_task
            .co_join()
            .await?
            .context("io error while joining a child process")
    }
}
