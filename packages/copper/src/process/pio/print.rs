
use tokio::process::{ChildStderr, ChildStdout};

use crate::print::Lv;
use crate::BoxedFuture;

use super::{ChildOutConfig, ChildOutTask, Command, Child, DriverOutput, Driver};

impl ChildOutConfig for Lv {
    type Task = PrintTask;

    fn configure_stdout(&mut self, command: &mut Command) {
        command.stdout(std::process::Stdio::piped());
    }

    fn configure_stderr(&mut self, command: &mut Command) {
        command.stderr(std::process::Stdio::piped());
    }

    fn take(self, child: &mut Child, name: Option<&str>, is_out: bool) -> crate::Result<Self::Task> {
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
                err: None
            })
        } else {
            Ok(PrintTask {
                lv: self,
                prefix,
                out: None,
                err: child.stderr.take()
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
        let mut driver = Driver::new(self.out, self.err);
        loop {
            match driver.next().await {
                DriverOutput::Line(line) => {
                    crate::__priv::__print_with_level(lv, format_args!("{prefix}{line}"));
                }
                DriverOutput::Done => break,
                _ => {}
            }
        }
    }
}
