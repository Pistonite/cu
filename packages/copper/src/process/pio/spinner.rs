use std::process::Stdio;
use std::sync::Arc;

use spin::mutex::SpinMutex;
use tokio::process::{Child as TokioChild, ChildStderr, ChildStdout, Command as TokioCommand};

use crate::{Atomic, BoxedFuture, ProgressBar, print::Lv};

use super::{ChildOutConfig, ChildOutTask, Driver, DriverOutput};

/// Display child process's status as a progress bar spinner.
///
/// # Example
/// Spawn a `git-clone` process, and use a spinner to show progress updates.
/// ```rust,no_run
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// cu::which("git")?.command()
///     .args(["clone", "--progress", "https://example1.git"])
///     // stdout should be empty, but if there are any messages,
///     // we will print them
///     .stdout(cu::lv::P)
///     // use spinner to show the bar
///     .stderr(cu::pio::spinner("cloning example1"))
///     .stdin_null()
///     .wait_nz()?;
/// # Ok(()) }
/// ```
///
/// # Behavior
/// The stream is splited into lines by `\r` and `\n` (`\r\n` is turned into single `\n`).
/// If the line ends with `\r`, it's considered a progress update.
///
/// By default, no matter if the line ends with `\r`, the bar will be updated
/// with the line. You can also configure the bar to only display
/// updates (end with `\r`), and print the other messages (end with `\n`)
/// as normal messages.
///
/// ```rust,no_run
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// cu::which("git")?.command()
///     .args(["clone", "--progress", "https://example1.git"])
///     // feed both stdout and stderr into the same bar
///     // when a progress update is done, it will also be printed as
///     // an info message
///     .stdoe(cu::pio::spinner("cloning example1").info())
///     .stdin_null()
///     .wait_nz()?;
/// # Ok(()) }
/// ```
///
/// # Merging streams
/// You can `clone` a spinner config to direct multiple streams to use the same
/// progress bar.
pub fn spinner(name: impl Into<String>) -> Spinner {
    Spinner {
        prefix: name.into(),
        config: Arc::new(SpinnerInner {
            lv: Atomic::new_u8(Lv::Off as u8),
            bar: SpinMutex::new(None),
        }),
    }
}

#[derive(Clone)]
#[doc(hidden)]
pub struct Spinner {
    /// prefix of the bar
    prefix: String,

    config: Arc<SpinnerInner>,
}
#[rustfmt::skip]
impl Spinner {
    /// Print any non-progress outputs as error messages
    pub fn error(self) -> Self { self.config.lv.set(crate::lv::E); self }
    /// Print any non-progress outputs as hint messages
    pub fn hint(self) -> Self { self.config.lv.set(crate::lv::H); self }
    /// Print any non-progress outputs as print messages
    pub fn print(self) -> Self { self.config.lv.set(crate::lv::P); self }
    /// Print any non-progress outputs as warning messages
    pub fn warn(self) -> Self { self.config.lv.set(crate::lv::W); self }
    /// Print any non-progress outputs as info messages
    pub fn info(self) -> Self { self.config.lv.set(crate::lv::I); self }
    /// Print any non-progress outputs as debug messages
    pub fn debug(self) -> Self { self.config.lv.set(crate::lv::D); self }
    /// Print any non-progress outputs as trace messages
    pub fn trace(self) -> Self { self.config.lv.set(crate::lv::T); self }
}
struct SpinnerInner {
    lv: Atomic<u8, Lv>,
    // the bar spawned when calling take() for the first time,
    // using a spin lock because it should be VERY rare that
    // we get contention
    bar: SpinMutex<Option<Arc<ProgressBar>>>,
}
pub struct SpinnerTask {
    lv: Lv,
    prefix: String,
    bar: Arc<ProgressBar>,
    out: Option<ChildStdout>,
    err: Option<ChildStderr>,
}
impl ChildOutConfig for Spinner {
    type Task = SpinnerTask;
    type __Null = super::__OCNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::piped());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::piped());
    }
    fn take(
        self,
        child: &mut TokioChild,
        name: Option<&str>,
        is_out: bool,
    ) -> crate::Result<Self::Task> {
        let lv = self.config.lv.get();
        let log_prefix = if crate::log_enabled(lv) {
            let name = name.unwrap_or_default();
            if name.is_empty() {
                String::new()
            } else {
                format!("[{name}] ")
            }
        } else {
            String::new()
        };
        let bar = {
            let mut bar_arc = self.config.bar.lock();
            if let Some(bar) = bar_arc.as_ref() {
                Arc::clone(bar)
            } else {
                let bar = crate::progress_unbounded(self.prefix);
                *bar_arc = Some(Arc::clone(&bar));
                bar
            }
        };
        Ok(SpinnerTask {
            lv,
            prefix: log_prefix,
            bar,
            out: if is_out { child.stdout.take() } else { None },
            err: if !is_out { child.stderr.take() } else { None },
        })
    }
}
impl ChildOutTask for SpinnerTask {
    type Output = ();

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        (Some(Box::pin(self.main())), ())
    }
}
impl SpinnerTask {
    async fn main(self) {
        let bar = self.bar;
        let lv = self.lv;
        let prefix = self.prefix;
        let mut driver = Driver::new(self.out, self.err, true);
        loop {
            match driver.next().await {
                DriverOutput::Line(line) => {
                    if lv != Lv::Off {
                        crate::__priv::__print_with_level(lv, format_args!("{prefix}{line}"));
                        // erase the progress line if we decide to print it out
                        crate::progress!(&bar, (), "")
                    } else {
                        crate::progress!(&bar, (), "{line}")
                    }
                }
                DriverOutput::Progress(line) => {
                    crate::progress!(&bar, (), "{line}")
                }
                DriverOutput::Done => break,
                _ => {}
            }
        }
    }
}
