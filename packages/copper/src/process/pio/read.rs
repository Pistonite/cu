use std::process::Stdio;

use tokio::io::{AsyncBufRead, AsyncBufReadExt as _, AsyncReadExt as _};
use tokio::process::{Child as TokioChild, ChildStderr, ChildStdout, Command as TokioCommand};

use crate::{BoxedFuture, Context as _, co};

use super::{ChildOutConfig, ChildOutTask};

/// Buffer the output as a `Vec<u8>`
///
/// # Example
/// ```rust
/// # use pistonite_cu as cu;
/// use cu::pre::*;
///
/// # #[cfg(unix)]
/// # fn main() -> cu::Result<()> {
/// let (hello, out) = cu::which("echo")?.command()
///     .arg("Hello, world!")
///     .stdout(cu::pio::buffer())
///     .stdie_null()
///     .spawn()?;
///
/// assert_eq!(b"Hello, world!\n".to_vec(), out.join()??);
/// # Ok(()) }
/// ```
pub fn buffer() -> Buffer {
    Buffer
}
pub struct Buffer;
impl ChildOutConfig for Buffer {
    type Task = BufferTask;
    type __Null = super::__OCNonNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::piped());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::piped());
    }
    fn take(
        self,
        child: &mut TokioChild,
        _: Option<&str>,
        is_out: bool,
    ) -> crate::Result<Self::Task> {
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
        let output: Self::Output =
            co::spawn(async move { recv.await.context("failed to receive output as bytes")? });
        let task = async move {
            let buf = read_to_end(self.0).await;
            let _: Result<_, _> = send.send(buf);
        };
        (Some(Box::pin(task)), output)
    }
}

/// Buffer the output as a `String`. Will error if the output is not valid UTF-8.
///
/// # Example
/// ```rust
/// # use pistonite_cu as cu;
/// use cu::pre::*;
///
/// # #[cfg(unix)]
/// # fn main() -> cu::Result<()> {
/// let (hello, out) = cu::which("echo")?.command()
///     .arg("Hello, world!")
///     .stdout(cu::pio::string())
///     .stdie_null()
///     .spawn()?;
///
/// assert_eq!("Hello, world!\n", out.join()??);
/// # Ok(()) }
/// ```
pub fn string() -> BufferString {
    BufferString
}
pub struct BufferString;
impl ChildOutConfig for BufferString {
    type Task = BufferStringTask;
    type __Null = super::__OCNonNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::piped());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::piped());
    }
    fn take(
        self,
        child: &mut TokioChild,
        _: Option<&str>,
        is_out: bool,
    ) -> crate::Result<Self::Task> {
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
        let output: Self::Output =
            co::spawn(async move { recv.await.context("failed to receive output as bytes")? });
        let task = async move {
            let string = read_to_end(self.0).await.and_then(|buf| {
                String::from_utf8(buf).context("failed to decode child output as utf-8")
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
        Err(mut r) => r.read_to_end(&mut buf).await,
    }
    .context("io error while reading output")?;
    Ok(buf)
}

/// Create reader to access output line-by-line.
///
/// The items will be available for read as soon as they are piped from the child,
/// without having to wait for the child to finish first.
///
/// Note that for small outputs, it's usually better to simply use [`string()`]
/// and call `.lines()` on the result.
///
/// # Example
/// ```rust
/// # use pistonite_cu as cu;
/// use cu::pre::*;
///
/// # #[cfg(unix)]
/// # fn main() -> cu::Result<()> {
/// let (child, mut lines) = cu::which("bash")?.command()
///     .args(["-c", r#"for i in {1..5}; do echo "Line $i"; done"# ])
///     .stdout(cu::pio::lines())
///     .stdie_null()
///     .spawn()?;
///
/// assert_eq!(lines.next().unwrap()?, "Line 1");
/// assert_eq!(lines.next().unwrap()?, "Line 2");
/// assert_eq!(lines.next().unwrap()?, "Line 3");
/// assert_eq!(lines.next().unwrap()?, "Line 4");
/// assert_eq!(lines.next().unwrap()?, "Line 5");
/// assert!(lines.next().is_none());
///
/// child.wait_nz()?;
/// # Ok(())}
/// ```
///
/// # Blocking
/// Since the iterator blocks the thread while waiting for the next line
/// to be available, use [`co_lines`] when in an async context.
pub fn lines() -> Lines {
    Lines
}
pub struct Lines;
impl ChildOutConfig for Lines {
    type Task = LinesTask;
    type __Null = super::__OCNonNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::piped());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::piped());
    }
    fn take(
        self,
        child: &mut TokioChild,
        _: Option<&str>,
        is_out: bool,
    ) -> crate::Result<Self::Task> {
        Ok(LinesTask(super::take_child_out(child, is_out)?))
    }
}

pub struct LinesTask(Result<ChildStdout, ChildStderr>);
impl ChildOutTask for LinesTask {
    type Output = LinesOutput;

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        let (send, recv) = std::sync::mpsc::channel();
        let output = LinesOutput { recv, done: false };
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

/// Output of [`lines`], a synchronous iterator that can reads the output line-by-line.
///
/// Just like the standard library's `lines()` iterator, the items have no line endings.
pub struct LinesOutput {
    recv: std::sync::mpsc::Receiver<Option<crate::Result<String>>>,
    done: bool,
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

/// Create reader to access output line-by-line.
///
/// The items will be available for read as soon as they are piped from the child,
/// without having to wait for the child to finish first.
///
/// Note that for small outputs, it's usually better to simply use [`string()`]
/// and call `.lines()` on the result.
///
/// # Example
/// ```rust
/// # use pistonite_cu as cu;
/// use cu::pre::*;
///
/// # #[cfg(unix)]
/// # #[cu::cli]
/// # async fn main(_: cu::cli::Flags) -> cu::Result<()> {
/// let (child, mut lines) = cu::which("bash")?.command()
///     .args(["-c", r#"for i in {1..5}; do echo "Line $i"; done"# ])
///     .stdout(cu::pio::co_lines())
///     .stdie_null()
///     .co_spawn().await?;
///
/// let mut i = 1;
/// while let Some(line) = lines.next().await {
///     let line = line?;
///     assert_eq!(line, format!("Line {i}"));
///     i+=1;
/// }
///
/// child.co_wait_nz().await?;
/// # Ok(())}
/// ```
pub fn co_lines() -> CoLines {
    CoLines
}
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
    fn take(
        self,
        child: &mut TokioChild,
        _: Option<&str>,
        is_out: bool,
    ) -> crate::Result<Self::Task> {
        Ok(CoLinesTask(super::take_child_out(child, is_out)?))
    }
}
pub struct CoLinesTask(Result<ChildStdout, ChildStderr>);
impl ChildOutTask for CoLinesTask {
    type Output = CoLinesOutput;

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        let (send, recv) = tokio::sync::mpsc::unbounded_channel();
        let output = CoLinesOutput { recv, done: false };
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

/// Output of [`co_lines`], an ansynchronous iterator that can reads the output line-by-line.
///
/// Just like the standard library's `lines()` iterator, the items have no line endings.
pub struct CoLinesOutput {
    recv: tokio::sync::mpsc::UnboundedReceiver<Option<crate::Result<String>>>,
    done: bool,
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
async fn read_send_line<R: AsyncBufRead + Unpin, S: LineSender>(r: R, send: S) {
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
