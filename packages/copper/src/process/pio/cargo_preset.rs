use std::process::Stdio;

use tokio::process::{Child as TokioChild, ChildStderr, ChildStdout, Command as TokioCommand};

use crate::lv::Lv;
use crate::BoxedFuture;
use crate::process::{Command, Preset, pio};

pub struct Cargo {
    prefix: String,
    error_level: Lv,
    warning_level: Lv,
    other_level: Lv,
}

pub fn cargo() -> Cargo {
    todo!();
}

impl Preset for Cargo {
    type Output = Command<Cargo, Cargo, pio::Null>;

    fn configure< O, E, I, >(self, command: crate::Command<O, E, I>) -> Self::Output {
        todo!()
    }
}
pub struct CargoTask {
    lv: Lv,
    prefix: String,
    // bar: Arc<ProgressBar>,
    out: ChildStdout,
    err: ChildStderr,
}

impl pio::ChildOutConfig for Cargo {
    type Task = CargoTask;
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
impl pio::ChildOutTask for CargoTask {
    type Output = ();

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        todo!()
        // (Some(Box::pin(self.main())), ())
    }
}
