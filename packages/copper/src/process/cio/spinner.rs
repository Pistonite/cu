use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::process::{ChildStderr, ChildStdout};

use crate::{print::Lv, BoxedFuture, ProgressBar};

use super::{ChildOutConfig, ChildTask, Command, Child, Driver, DriverOutput};

#[derive(Clone)]
#[doc(hidden)]
pub struct Spinner(String, Arc<SpinnerConfig>);
#[rustfmt::skip]
impl Spinner {
    /// Print any non-progress outputs as error messages
    pub fn error(self) -> Self { self.1.lv.set(crate::lv::E); self }
    /// Print any non-progress outputs as hint messages
    pub fn hint(self) -> Self { self.1.lv.set(crate::lv::H); self }
    /// Print any non-progress outputs as print messages
    pub fn print(self) -> Self { self.1.lv.set(crate::lv::P); self }
    /// Print any non-progress outputs as warning messages
    pub fn warn(self) -> Self { self.1.lv.set(crate::lv::W); self }
    /// Print any non-progress outputs as info messages
    pub fn info(self) -> Self { self.1.lv.set(crate::lv::I); self }
    /// Print any non-progress outputs as debug messages
    pub fn debug(self) -> Self { self.1.lv.set(crate::lv::D); self }
    /// Print any non-progress outputs as trace messages
    pub fn trace(self) -> Self { self.1.lv.set(crate::lv::T); self }
}
struct SpinnerConfig {
    lv: atomic::AtomicU8<Lv>,
    out: AtomicBool,
    err: AtomicBool,
}
#[doc(hidden)]
pub struct SpinnerTask {
    lv: Lv,
    prefix: String,
    bar: Arc<ProgressBar>,
    out: Option<ChildStdout>,
    err: Option<ChildStderr>,
}
pub fn spinner(name: impl Into<String>) -> Spinner { 
    Spinner(name.into(), Arc::new(SpinnerConfig { 
        lv: atomic::AtomicU8::new(Lv::Off as u8),
        out: AtomicBool::new(false), err: AtomicBool::new(false) 
    }))
}
impl ChildOutConfig for Spinner {
    type Output = SpinnerTask;
    fn configure_stdout(&mut self, command: &mut Command) {
        command.stdout(std::process::Stdio::piped());
        self.1.out.store(true, Ordering::Release);
    }
    fn configure_stderr(&mut self, command: &mut Command) {
        command.stderr(std::process::Stdio::piped());
        self.1.err.store(true, Ordering::Release);
    }
    fn set_name(&mut self, name: &str) {
        self.0 = name.to_string();
    }
    fn take(self, child: &mut Child, _: bool) -> Self::Output {
        let out = self.1.out.load(Ordering::Acquire);
        let err = self.1.err.load(Ordering::Acquire);
        let lv= self.1.lv.get();
        let prefix = if crate::log_enabled(lv) {
            if self.0.is_empty() {
                String::new()
            } else {
                format!("[{}] ", self.0)
            }
        } else {
            String::new()
        };
        let bar = crate::progress_unbounded(self.0);
        SpinnerTask {
            lv,
            prefix,
            bar,
            out: if out { child.stdout.take() } else { None },
            err: if err { child.stderr.take() } else { None },
        }
    }
}
impl ChildTask for SpinnerTask {
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
        let mut driver = Driver::new(self.out, self.err);
        loop {
            match driver.next().await {
                DriverOutput::Line(line) => {
                    if lv != Lv::Off {
                        crate::__priv::__print_with_level(lv, format_args!("{prefix}{line}"));
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
