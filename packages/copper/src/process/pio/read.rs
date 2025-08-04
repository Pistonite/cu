

use std::process::Stdio;

use tokio::io::{AsyncBufRead, AsyncRead, AsyncReadExt, AsyncBufReadExt as _};
use tokio::process::{Child as TokioChild, Command as TokioCommand, ChildStdout, ChildStderr};

use crate::{BoxedFuture, Context as _, co};

use super::{ChildOutConfig, ChildInConfig, ChildOutTask};

pub struct Buffer;
pub struct BufferString;

// capture the output as Vec<u8>
pub fn buffer() -> Buffer { Buffer }

impl ChildOutConfig for Buffer {
    type Task = BufferTask;
    type __Null = super::__OCNonNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::piped());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::piped());
    }
    fn take(self, child: &mut TokioChild, _: Option<&str>, is_out: bool) -> crate::Result<Self::Task> {
        Ok(BufferTask(super::take_child_out(child, is_out)?))
    }
}

pub struct BufferTask(Result<ChildStdout, ChildStderr>);
impl ChildOutTask for BufferTask {
    type Output = co::Handle<crate::Result<Vec<u8>>>;

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        // we use a channel instead of returning the future in the output
        // slot because we want to drive it internally to be less error-prone.
        // if we return the future in the second slot without `spawn`,
        // there could be a scenario where the future is never polled,
        // resulting in stdout being stuck
        let (send, recv) = tokio::sync::oneshot::channel();
        let output: Self::Output = co::spawn(async move {
            recv.await.context("failed to receive output as bytes")?
        });
        let task = async move {
            let buf = read_to_end(self.0).await;
            let _: Result<_, _> = send.send(buf);
        };
        (Some(Box::pin(task)), output)
    }
}

pub fn string() -> BufferString { BufferString }
impl ChildOutConfig for BufferString {
    type Task = BufferStringTask;
    type __Null = super::__OCNonNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::piped());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::piped());
    }
    fn take(self, child: &mut TokioChild, _: Option<&str>, is_out: bool) -> crate::Result<Self::Task> {
        Ok(BufferStringTask(super::take_child_out(child, is_out)?))
    }
}

pub struct BufferStringTask(Result<ChildStdout, ChildStderr>);
impl ChildOutTask for BufferStringTask {
    type Output = co::Handle<crate::Result<String>>;

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        // we use a channel instead of returning the future in the output
        // slot because we want to drive it internally to be less error-prone.
        // if we return the future in the second slot without `spawn`,
        // there could be a scenario where the future is never polled,
        // resulting in stdout being stuck
        let (send, recv) = tokio::sync::oneshot::channel();
        let output: Self::Output = co::spawn(async move {
            recv.await.context("failed to receive output as bytes")?
        });
        let task = async move {
            let string = read_to_end(self.0).await
                .and_then(|buf| {
                    String::from_utf8(buf)
                        .context("failed to decode child output as utf-8")
                });
            let _: Result<_, _> = send.send(string);
        };
        (Some(Box::pin(task)), output)
    }
}

async fn read_to_end(r: Result<ChildStdout, ChildStderr>) -> crate::Result<Vec<u8>> {
    let mut buf = Vec::new();
    match r {
        Ok(mut r) => r.read_to_end(&mut buf).await,
        Err(mut r) => r.read_to_end(&mut buf).await
    }.context("io error while reading output")?;
    Ok(buf)
}

pub struct Lines;

pub fn lines() -> Lines { Lines }
impl ChildOutConfig for Lines {
    type Task = LinesTask;
    type __Null = super::__OCNonNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::piped());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::piped());
    }
    fn take(self, child: &mut TokioChild, _: Option<&str>, is_out: bool) -> crate::Result<Self::Task> {
        Ok(LinesTask(super::take_child_out(child, is_out)?))
    }
}

pub struct LinesTask(Result<ChildStdout, ChildStderr>);
impl ChildOutTask for LinesTask {
    type Output = LinesOutput;

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        let (send, recv) = std::sync::mpsc::channel();
        let output = LinesOutput {
            recv, done: false
        };
        (Some(Box::pin(self.main(send))), output)
    }
}

impl LinesTask {
    async fn main(self, send: std::sync::mpsc::Sender<Option<crate::Result<String>>>) {
        match self.0 {
            Ok(r) => read_send_line(tokio::io::BufReader::new(r), send).await,
            Err(r) => read_send_line(tokio::io::BufReader::new(r), send).await,
        }
    }
}

pub struct LinesOutput{
    recv: std::sync::mpsc::Receiver<Option<crate::Result<String>>>,
    done:bool
    }
impl Iterator for LinesOutput {
    type Item = crate::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.recv.recv() {
            Ok(Some(x)) => {
                if x.is_err() {
                    self.done = true;
                }
                Some(x)
            }
            // no more data
            _ => {
                self.done = true;
                None
            }
        }
    }
}

pub fn co_lines() -> CoLines { CoLines }
pub struct CoLines;
impl ChildOutConfig for CoLines {
    type Task = CoLinesTask;
    type __Null = super::__OCNonNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::piped());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::piped());
    }
    fn take(self, child: &mut TokioChild, _: Option<&str>, is_out: bool) -> crate::Result<Self::Task> {
        Ok(CoLinesTask(super::take_child_out(child, is_out)?))
    }
}
pub struct CoLinesTask(Result<ChildStdout, ChildStderr>);
impl ChildOutTask for CoLinesTask {
    type Output = CoLinesOutput;

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        let (send, recv) = tokio::sync::mpsc::unbounded_channel();
        let output = CoLinesOutput {
            recv, done: false
        };
        (Some(Box::pin(self.main(send))), output)
    }
}

impl CoLinesTask {
    async fn main(self, send: tokio::sync::mpsc::UnboundedSender<Option<crate::Result<String>>>) {
        match self.0 {
            Ok(r) => read_send_line(tokio::io::BufReader::new(r), send).await,
            Err(r) => read_send_line(tokio::io::BufReader::new(r), send).await,
        }
    }
}

pub struct CoLinesOutput{
    recv: tokio::sync::mpsc::UnboundedReceiver<Option<crate::Result<String>>>,
    done: bool
}
impl CoLinesOutput {
    /// Get the next line
    pub async fn next(&mut self) -> Option<crate::Result<String>> {
        if self.done {
            return None;
        }
        match self.recv.recv().await {
            Some(Some(x)) => {
                if x.is_err() {
                    self.done = true;
                }
                Some(x)
            }
            // no more data
            _ => {
                self.done = true;
                None
            }
        }
    }
}

trait LineSender {
    fn send(&self, payload: Option<crate::Result<String>>) -> bool;
}
impl LineSender for std::sync::mpsc::Sender<Option<crate::Result<String>>> {
    fn send(&self, payload: Option<crate::Result<String>>) -> bool {
        self.send(payload).is_ok()
    }
}
impl LineSender for tokio::sync::mpsc::UnboundedSender<Option<crate::Result<String>>> {
    fn send(&self, payload: Option<crate::Result<String>>) -> bool {
        self.send(payload).is_ok()
    }
}
async fn read_send_line<
R: AsyncBufRead + Unpin,
S: LineSender
>(r: R, send: S) {
    let mut lines = r.lines();
    loop {
        match lines.next_line().await {
            Err(e) => {
                // io error
                let payload = Err(e).context("error reading line from child output");
                send.send(Some(payload));
                return;
            }
            Ok(Some(x)) => {
                if !send.send(Some(Ok(x))) {
                    // try sending the end message
                    send.send(None);
                    return;
                }
            }
            Ok(None) => {
                // no more lines
                let _ = send.send(None);
                return;
            }
        }
    }
}
