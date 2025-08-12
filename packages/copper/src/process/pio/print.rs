use std::process::Stdio;

use tokio::process::{Child as TokioChild, ChildStderr, ChildStdout, Command as TokioCommand};

use crate::BoxedFuture;
use crate::lv::Lv;

use super::{ChildOutConfig, ChildOutTask, Driver, DriverOutput};

impl ChildOutConfig for Lv {
    type Task = PrintTask;
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
        let name = name.unwrap_or_default();
        let prefix = if !name.is_empty() {
            format!("[{name}] ")
        } else {
            String::new()
        };

        if is_out {
            Ok(PrintTask {
                lv: self,
                prefix,
                out: child.stdout.take(),
                err: None,
            })
        } else {
            Ok(PrintTask {
                lv: self,
                prefix,
                out: None,
                err: child.stderr.take(),
            })
        }
    }
}

pub struct PrintTask {
    lv: Lv,
    prefix: String,
    out: Option<ChildStdout>,
    err: Option<ChildStderr>,
}
impl ChildOutTask for PrintTask {
    type Output = ();

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        (Some(Box::pin(self.main())), ())
    }
}
impl PrintTask {
    async fn main(self) {
        let lv = self.lv;
        let prefix = self.prefix;
        let mut driver = Driver::new(self.out, self.err, false);
        loop {
            match driver.next().await {
                DriverOutput::Line(line) => {
                    for l in line.lines() {
                        crate::__priv::__print_with_level(lv, format_args!("{prefix}{l}"));
                    }
                }
                DriverOutput::Done => break,
                _ => {}
            }
        }
    }
}
