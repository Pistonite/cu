
use tokio::process::{ChildStderr, ChildStdout};

use crate::print::Lv;
use crate::BoxedFuture;

use super::{ChildOutConfig, ChildTask, Command, Child, DriverOutput, Driver};

impl ChildOutConfig for Lv {
    type Output = PrintTask;

    fn configure_stdout(&mut self, command: &mut Command) {
        command.stdout(std::process::Stdio::piped());
    }

    fn configure_stderr(&mut self, command: &mut Command) {
        command.stderr(std::process::Stdio::piped());
    }

    fn set_name(&mut self, name: &str) {
        todo!()
        
    }

    fn take(self, child: &mut Child, _: bool) -> Self::Output {
        todo!()
    }

}

pub struct PrintTask {
    lv: Lv,
    prefix: String,
    out: Option<ChildStdout>,
    err: Option<ChildStderr>,
}
impl ChildTask for PrintTask {
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
