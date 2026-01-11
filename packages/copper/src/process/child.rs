use std::{process::ExitStatus, time::Duration};

use tokio::{process::Child as TokioChild, task::JoinSet};

use crate::{BoxedFuture, Context as _, co};

/// A child process spawned with [`Command`](crate::Command)
pub struct Child {
    pub(crate) name: String,
    pub(crate) inner: TokioChild,
    pub(crate) io: ChildIo,
}

impl Child {
    /// Block the thread and wait for the child to finish, and check if the ExitStatus is 0
    ///
    /// Note that currently any error that occurred in input/output tasks are treated
    /// as warning and not hard error.
    ///
    /// # Blocking
    /// This will block the current thread while trying to join the child.
    /// Use [`co_wait_nz`](Self::co_wait_nz) to avoid blocking if in async context.
    pub fn wait_nz(self) -> crate::Result<()> {
        let status = wait_internal(&self.name, self.inner, self.io)?;
        if !status.success() {
            crate::bail!("{} exited with non-zero status", self.name);
        }
        Ok(())
    }

    /// Block the thread and wait for the child to finish,
    /// and return its [`ExitStatus`]
    ///
    /// Note that currently any error that occurred in input/output tasks are treated
    /// as warning and not hard error.
    ///
    /// # Blocking
    /// This will block the current thread while trying to join the child.
    /// Use [`co_wait`](Self::co_wait) to avoid blocking if in async context.
    #[inline(always)]
    pub fn wait(self) -> crate::Result<ExitStatus> {
        wait_internal(&self.name, self.inner, self.io)
    }

    /// Wait for the child asynchronously using the current tokio runtime,
    /// and check if the ExitStatus is 0.
    ///
    /// Note that currently any error that occurred in input/output tasks are treated
    /// as warning and not hard error.
    ///
    /// # Panic
    /// Will panic if called outside of a tokio runtime context
    pub async fn co_wait_nz(self) -> crate::Result<()> {
        let status = co_wait_internal(&self.name, self.inner, self.io).await?;
        if !status.success() {
            crate::bail!("{} exited with non-zero status", self.name);
        }
        Ok(())
    }

    /// Wait for the child asynchronously using the current tokio runtime,
    /// and return its [`ExitStatus`]
    ///
    /// Note that currently any error that occurred in input/output tasks are treated
    /// as warning and not hard error.
    ///
    /// # Panic
    /// Will panic if called outside of a tokio runtime context
    #[inline(always)]
    pub async fn co_wait(self) -> crate::Result<ExitStatus> {
        co_wait_internal(&self.name, self.inner, self.io).await
    }

    /// Create a wait guard that will automatically wait for the child
    /// (and ignore the error) when going out of scope.
    #[inline(always)]
    pub fn wait_guard(self) -> ChildWaitGuard {
        ChildWaitGuard { inner: Some(self) }
    }

    /// Wait for the child to exit for at least the timeout.
    /// Return the exit status if exited
    pub async fn co_wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> crate::Result<Option<ExitStatus>> {
        let mut ms = 100;
        let mut total_ms = 0;
        loop {
            match self.inner.try_wait() {
                Ok(Some(s)) => return Ok(Some(s)),
                Ok(None) => {}
                Err(e) => {
                    crate::rethrow!(e, "io error while waiting {}", self.name)
                }
            }
            total_ms += ms;
            if Duration::from_millis(total_ms) >= timeout {
                break;
            }
            tokio::time::sleep(Duration::from_millis(ms)).await;
            ms *= 4;
        }
        Ok(None)
    }

    pub fn wait_timeout(&mut self, timeout: Duration) -> crate::Result<Option<ExitStatus>> {
        let mut ms = 100;
        let mut total_ms = 0;
        loop {
            match self.inner.try_wait() {
                Ok(Some(s)) => return Ok(Some(s)),
                Ok(None) => {}
                Err(e) => {
                    crate::rethrow!(e, "io error while waiting {}", self.name)
                }
            }
            std::thread::sleep(Duration::from_millis(ms));
            total_ms += ms;
            if Duration::from_millis(total_ms) >= timeout {
                break;
            }
            ms *= 4;
        }
        Ok(None)
    }

    /// Kill the child and wait for it to exit asynchronously. Return the exit status
    ///
    /// # Panic
    /// Will panic if called outside of a tokio runtime context
    pub async fn co_kill(mut self) -> crate::Result<ExitStatus> {
        let mut ms = 100;
        for i in 0..5 {
            crate::trace!("trying to kill child '{}', attempt {}", self.name, i + 1);
            crate::check!(
                self.inner.start_kill(),
                "failed to send kill signal to child"
            )?;
            match self.inner.try_wait() {
                Ok(Some(s)) => return Ok(s),
                Ok(None) => {}
                Err(e) => {
                    crate::rethrow!(e, "io error while killing {}", self.name)
                }
            }
            tokio::time::sleep(Duration::from_millis(ms)).await;
            ms *= 4;
        }
        self.io.co_join(&self.name).await;
        crate::bail!("failed to kill child '{}' after many attempts", self.name);
    }

    /// Kill the child and block the current thread until the child exits. Return the exit status.
    ///
    /// # Blocking
    /// This will block the current thread while trying to join the child.
    /// Use [`co_kill`](Self::co_kill) to avoid blocking if in async context.
    pub fn kill(mut self) -> crate::Result<ExitStatus> {
        let mut ms = 100;
        for i in 0..5 {
            crate::trace!("trying to kill child '{}', attempt {}", self.name, i + 1);
            crate::check!(
                self.inner.start_kill(),
                "failed to send kill signal to child"
            )?;
            match self.inner.try_wait() {
                Ok(Some(s)) => return Ok(s),
                Ok(None) => {}
                Err(e) => {
                    crate::rethrow!(e, "io error while killing {}", self.name)
                }
            }
            std::thread::sleep(Duration::from_millis(ms));
            ms *= 4;
        }
        self.io.join(&self.name);
        crate::bail!("failed to kill child '{}' after many attempts", self.name);
    }
}

fn wait_internal(name: &str, mut child: TokioChild, io: ChildIo) -> crate::Result<ExitStatus> {
    // consume the child by waiting
    let wait_task = co::spawn(async move { child.wait().await });
    // ensure the IO tasks are finished first, since blocking
    // on child could dead lock if the child is waiting for IO
    io.join(name);
    crate::check!(wait_task.join()?, "io error while executing {name}")
}

async fn co_wait_internal(
    name: &str,
    mut child: TokioChild,
    io: ChildIo,
) -> crate::Result<ExitStatus> {
    // consume the child by waiting
    let wait_task = co::spawn(async move { child.wait().await });
    // ensure the IO tasks are finished first, since blocking
    // on child could dead lock if the child is waiting for IO
    io.co_join(name).await;
    crate::check!(
        wait_task.co_join().await?,
        "io error while executing {name}"
    )
}

/// IO Task for a child
pub(crate) struct ChildIo {
    // (panicked, stdin_err)
    inner: co::Handle<(bool, crate::Result<()>)>,
}
impl ChildIo {
    pub fn start(
        stdin: Option<BoxedFuture<crate::Result<()>>>,
        stdout: Option<BoxedFuture<()>>,
        stderr: Option<BoxedFuture<()>>,
    ) -> Self {
        let combined_future = async move {
            let mut j = JoinSet::new();
            if let Some(x) = stdin {
                j.spawn(x);
            }
            if let Some(x) = stdout {
                j.spawn(async move {
                    x.await;
                    Ok(())
                });
            }
            if let Some(x) = stderr {
                j.spawn(async move {
                    x.await;
                    Ok(())
                });
            }

            let mut panicked = false;
            let mut stdin_err = Ok(());
            while let Some(x) = j.join_next().await {
                match x {
                    // could not join because panicked
                    Err(_) => panicked = true,
                    Ok(Err(e)) => {
                        // this must be stdin because
                        // the out tasks return ()
                        stdin_err = Err(e);
                    }
                    Ok(Ok(())) => {}
                }
            }
            (panicked, stdin_err)
        };
        let io_task = co::spawn(combined_future);
        Self { inner: io_task }
    }

    pub fn join(self, name: &str) {
        Self::do_join(name, self.inner.join_maybe_aborted())
    }

    pub async fn co_join(self, name: &str) {
        Self::do_join(name, self.inner.co_join_maybe_aborted().await)
    }

    fn do_join(name: &str, result: crate::Result<Option<(bool, crate::Result<()>)>>) {
        match result {
            Ok(Some((panicked, stdin_err))) => {
                if let Err(e) = stdin_err {
                    crate::warn!("failed to write to stdin while executing {name}: {e:?}")
                }
                if panicked {
                    crate::warn!("some io tasks panicked while executing {name}");
                }
            }
            Ok(None) => {
                // aborted
            }
            Err(_) => {
                crate::warn!("some io tasks panicked while executing {name}");
            }
        }
    }
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
