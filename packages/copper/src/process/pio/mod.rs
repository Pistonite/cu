use std::{io::Cursor, sync::{atomic::{AtomicBool, Ordering}, Arc}};

use crate::{print::Lv, BoxedFuture, ProgressBar};

use super::{Command, Child};

mod spinner;
pub use spinner::*;
mod print;

mod print_driver;
use print_driver::*;

macro_rules! ConfiguredChild {
    ($In:ident, $Out:ident, $Err:ident) => {
        $crate::process::spawned::LwChild<
            <$In as $crate::process::pio::ChildInConfig>::Output,
            <$Out as $crate::process::pio::ChildOutConfig>::Output,
            <$Err as $crate::process::pio::ChildOutConfig>::Output
        >
        
    };
}
pub(crate) use ConfiguredChild;

pub trait ChildOutConfig {
    type Output: ChildTask;
    /// Configure the standard output using this config
    fn configure_stdout(&mut self, command: &mut Command);
    fn configure_stderr(&mut self, command: &mut Command);
    fn set_name(&mut self, name: &str);
    fn take(self, child: &mut Child, is_out: bool) -> Self::Output;
}
pub trait ChildInConfig {
    type Output: ChildTask;
    /// Configure the standard input using this config
    fn configure_stdin(&mut self, command: &mut Command);
    fn take(self, child: &mut Child) -> Self::Output;
}
pub trait ChildTask {
    type Output;
    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output);
}
impl ChildTask for () {
    type Output = ();
    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        (None, ())
    }
}

#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct Inherit;
/// Inherit the parent's stdin, stdout, or stderr.
pub fn raw_inherit() -> Inherit { Inherit }
impl ChildOutConfig for Inherit {
    type Output = ();
    fn configure_stdout(&mut self, command: &mut Command) {
        command.stdout(std::process::Stdio::inherit());
    }
    fn configure_stderr(&mut self, command: &mut Command) {
        command.stderr(std::process::Stdio::inherit());
    }
    fn set_name(&mut self, _: &str) {}
    fn take(self, _: &mut Child, _: bool) {}
}
impl ChildInConfig for Inherit {
    type Output = ();
    fn configure_stdin(&mut self, command: &mut Command) -> Self::Output {
        command.stdin(std::process::Stdio::inherit());
    }
    fn take(self, _: &mut Child) {}
}
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct Null;
/// Direct the stream to null (i.e. ignore it)
pub fn null() -> Null { Null }
impl ChildOutConfig for Null {
    type Output = ();
    fn configure_stdout(&mut self, command: &mut Command) -> Self::Output {
        command.stdout(std::process::Stdio::null());
    }
    fn configure_stderr(&mut self, command: &mut Command) -> Self::Output {
        command.stderr(std::process::Stdio::null());
    }
    fn set_name(&mut self, _: &str) {}
    fn take(self, _: &mut Child, _: bool) {}
}
impl ChildInConfig for Null {
    type Output = ();
    fn configure_stdin(&mut self, command: &mut Command) -> Self::Output {
        command.stdin(std::process::Stdio::null());
    }
    fn take(self, _: &mut Child) {}
}

