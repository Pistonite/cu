use std::{io::Cursor, sync::{atomic::{AtomicBool, Ordering}, Arc}};

use crate::{print::Lv, BoxedFuture, ProgressBar};

use super::{Command, Child};

mod spinner;
pub use spinner::*;
mod print;
mod pipe;
pub use pipe::*;

mod print_driver;
use print_driver::*;

macro_rules! ConfiguredChild {
    ($Out:ident, $Err:ident) => {
        $crate::process::spawned::LwChild<
            <$Out as $crate::process::pio::ChildOutConfig>::Task,
            <$Err as $crate::process::pio::ChildOutConfig>::Task
        >
        
    };
}
pub(crate) use ConfiguredChild;

pub trait ChildOutConfig: Send + 'static {
    type Task: ChildOutTask;
    /// Configure the standard output using this config, called before spawning
    fn configure_stdout(&mut self, command: &mut Command);
    /// Configure the standard error using this config, called before spawning
    fn configure_stderr(&mut self, command: &mut Command);

    // === once tokio exposes a way for us to take from StdChild, this could be
    // used to optimize pipes
    // /// Take the bits needed for this out config from the child, but operating on std
    // ///
    // /// If `Err` or `Ok(Some)` is returned, then `take()` will not be called,
    // /// and it will be safe to implement it as `!unreachable()`
    // fn take_std(self, _child: &mut std::process::Child, _is_out: bool) -> crate::Result<Option<Self::Task>> where Self: Sized {
    //     Ok(None)
    // }

    /// Take the bits needed for this out config from the child
    fn take(self, child: &mut Child, name: Option<&str>, is_out: bool) -> crate::Result<Self::Task>;
}
pub trait ChildInConfig: Send + 'static {
    type Task: ChildInTask;
    /// Configure the standard input using this config
    fn configure_stdin(&mut self, command: &mut Command) -> crate::Result<()>;
    /// Take the bits needed for this in config from the child
    fn take(self, child: &mut Child) -> crate::Result<Self::Task>;
}
pub trait ChildOutTask {
    type Output: Send + 'static;
    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output);
}
pub trait ChildInTask {
    fn run(self) -> Option<BoxedFuture<()>>;
}
impl ChildOutTask for () {
    type Output = ();
    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        (None, ())
    }
}
impl ChildInTask for () {
    fn run(self) -> Option<BoxedFuture<()>> {
        None
    }
}

#[derive(Clone, Copy)]
pub struct Inherit;
/// Inherit the parent's stdin, stdout, or stderr.
pub fn inherit() -> Inherit { Inherit }
impl ChildOutConfig for Inherit {
    type Task = ();
    fn configure_stdout(&mut self, command: &mut Command) {
        command.stdout(std::process::Stdio::inherit());
    }
    fn configure_stderr(&mut self, command: &mut Command) {
        command.stderr(std::process::Stdio::inherit());
    }
    fn take(self, _: &mut Child, _: Option<&str>, _: bool) -> crate::Result<()> { Ok(()) }
}
impl ChildInConfig for Inherit {
    type Task = ();
    fn configure_stdin(&mut self, command: &mut Command) -> crate::Result<()> {
        command.stdin(std::process::Stdio::inherit());
        Ok(())
    }
    fn take(self, _: &mut Child) -> crate::Result<()> { Ok(()) }
}
#[derive(Clone, Copy)]
pub struct Null;
/// Direct the stream to null (i.e. ignore it)
pub fn null() -> Null { Null }
impl ChildOutConfig for Null {
    type Task = ();
    fn configure_stdout(&mut self, command: &mut Command) {
        command.stdout(std::process::Stdio::null());
    }
    fn configure_stderr(&mut self, command: &mut Command) {
        command.stderr(std::process::Stdio::null());
    }
    fn take(self, _: &mut Child, _: Option<&str>, _: bool) -> crate::Result<()> { Ok(()) }
}
impl ChildInConfig for Null {
    type Task = ();
    fn configure_stdin(&mut self, command: &mut Command) -> crate::Result<()> {
        command.stdin(std::process::Stdio::null());
        Ok(())
    }
    fn take(self, _: &mut Child) -> crate::Result<()> { Ok(()) }
}

