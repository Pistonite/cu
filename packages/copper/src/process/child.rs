use std::process::ExitStatus;

use tokio::process::Child as TokioChild;

use crate::{Context as _, co};

/// A child process spawned with [`Command`](crate::Command)
pub struct Child {
    pub(crate) name: String,
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
        let status = wait_internal(&self.name, self.inner, self.io_task)?;
        if !status.success() {
            crate::bail!("{} exited with non-zero status", self.name);
        }
        Ok(())
    }

    /// Block the thread and wait for the child to finish,
    /// and return its [`ExitStatus`]
    ///
    /// # Blocking
    /// This will block the current thread while trying to join the child.
    /// Use [`co_wait`](Self::co_wait) to avoid blocking if in async context.
    #[inline(always)]
    pub fn wait(self) -> crate::Result<ExitStatus> {
        wait_internal(&self.name, self.inner, self.io_task)
    }

    /// Wait for the child asynchronously using the current tokio runtime,
    /// and check if the ExitStatus is 0.
    ///
    /// # Panic
    /// Will panic if called outside of a tokio runtime context
    pub async fn co_wait_nz(self) -> crate::Result<()> {
        let status = co_wait_internal(&self.name, self.inner, self.io_task).await?;
        if !status.success() {
            crate::bail!("{} exited with non-zero status", self.name);
        }
        Ok(())
    }

    /// Wait for the child asynchronously using the current tokio runtime,
    /// and return its [`ExitStatus`]
    ///
    /// # Panic
    /// Will panic if called outside of a tokio runtime context
    #[inline(always)]
    pub async fn co_wait(self) -> crate::Result<ExitStatus> {
        co_wait_internal(&self.name, self.inner, self.io_task).await
    }

    /// Create a wait guard that will automatically wait for the child
    /// (and ignore the error) when going out of scope.
    #[inline(always)]
    pub fn wait_guard(self) -> ChildWaitGuard {
        ChildWaitGuard { inner: Some(self) }
    }
}

fn wait_internal(
    name: &str,
    mut child: TokioChild,
    io_task: co::Handle<bool>,
) -> crate::Result<ExitStatus> {
    // consume the child by waiting
    let wait_task = co::spawn(async move { child.wait().await });
    // ensure the IO tasks are finished first, since blocking
    // on child could dead lock if the child is waiting for IO
    let io_panicked = match io_task.join_maybe_aborted() {
        Ok(Some(panicked)) => panicked,
        Ok(None) => false, // aborted
        Err(_) => true,
    };
    if io_panicked {
        crate::warn!("some io tasks panicked while executing {name}");
    }
    crate::check!(wait_task.join()?, "io error while executing {name}")
}

async fn co_wait_internal(
    name: &str,
    mut child: TokioChild,
    io_task: co::Handle<bool>,
) -> crate::Result<ExitStatus> {
    // consume the child by waiting
    let wait_task = co::spawn(async move { child.wait().await });
    // ensure the IO tasks are finished first, since blocking
    // on child could dead lock if the child is waiting for IO
    let io_panicked = match io_task.co_join_maybe_aborted().await {
        Ok(Some(panicked)) => panicked,
        Ok(None) => false, // aborted
        Err(_) => true,
    };
    if io_panicked {
        crate::warn!("some io tasks panicked while executing {name}");
    }
    crate::check!(
        wait_task.co_join().await?,
        "io error while executing {name}"
    )
}

/// A guard that automatically calls `wait` on a child
/// when dropped. The result of the `wait` is ignored.
///
/// This can be constructed with [`Child::wait_guard`]
pub struct ChildWaitGuard {
    inner: Option<Child>,
}
impl Drop for ChildWaitGuard {
    fn drop(&mut self) {
        let Some(child) = self.inner.take() else {
            return;
        };
        match child.wait() {
            Err(e) => crate::trace!("wait guard: error while waiting for child: {e}"),
            Ok(x) => crate::trace!("wait guard: child exited with status: {x}"),
        };
    }
}
impl ChildWaitGuard {
    /// Call [`wait_nz`](Child::wait_nz) on the inner child
    pub fn wait_nz(mut self) -> crate::Result<()> {
        self.inner.take().unwrap().wait_nz()
    }
    /// Call [`wait`](Child::wait) on the inner child
    pub fn wait(mut self) -> crate::Result<ExitStatus> {
        self.inner.take().unwrap().wait()
    }
}
