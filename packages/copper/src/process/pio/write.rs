use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tokio::process::{Child as TokioChild, ChildStdin, Command as TokioCommand};

use crate::{BoxedFuture, Context as _};

use super::{ChildInConfig, ChildInTask};

/// Write an in-memory buffer to child's stdin.
///
/// Budgeting and scheduling of the write task is up to Tokio.
///
/// # Example
/// ```rust
/// # use pistonite_cu as cu;
/// use cu::pre::*;
///
/// # #[cfg(unix)]
/// # fn main() -> cu::Result<()> {
/// let (hello, out) = cu::which("grep")?.command()
///     .args(["foo", "-"])
///     .stdin(cu::pio::write("foobar\nhello"))
///     .stdout(cu::pio::buffer())
///     .stderr_null()
///     .spawn()?;
///
/// assert_eq!(b"foobar\n".to_vec(), out.join()??);
/// # Ok(()) }
/// ```
#[inline(always)]
pub fn write<B: AsRef<[u8]> + Send + 'static>(buf: B) -> Write<B> {
    Write(buf)
}
pub struct Write<B>(B);
impl<B: AsRef<[u8]> + Send + 'static> ChildInConfig for Write<B> {
    type Task = WriteTask<B>;

    fn configure_stdin(&mut self, command: &mut TokioCommand) -> crate::Result<()> {
        command.stdin(Stdio::piped());
        Ok(())
    }

    fn take(self, child: &mut TokioChild) -> crate::Result<Self::Task> {
        let stdin = crate::check!(child.stdin.take(), "unexpected: pipe was already taken")?;
        Ok(WriteTask { buf: self.0, stdin })
    }
}
pub struct WriteTask<B> {
    buf: B,
    stdin: ChildStdin,
}
impl<B: AsRef<[u8]> + Send + 'static> ChildInTask for WriteTask<B> {
    fn run(mut self) -> Option<BoxedFuture<crate::Result<()>>> {
        Some(Box::pin(async move {
            Ok(self.stdin.write_all(self.buf.as_ref()).await?)
        }))
    }
}
