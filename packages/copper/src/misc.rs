use std::any::Any;

use crate::Context as _;

/// Try to get info from a panic payload
pub fn best_effort_panic_info<'a>(payload: &'a Box<dyn Any + Send + 'static>) -> &'a str {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.as_str()
    } else {
        crate::debug!(
            "encountered unknown panic info with type id: {:?}",
            (**payload).type_id()
        );
        "unknown panic info"
    }
}

/// Copy a reader to a writer.
///
/// This is wrapper for `std::io::copy` with error context
#[inline(always)]
pub fn copy<R, W>(reader: &mut R, writer: &mut W) -> crate::Result<u64>
where
    R: std::io::Read + ?Sized,
    W: std::io::Write + ?Sized,
{
    std::io::copy(reader, writer).context("stream copy failed")
}

/// Copy an async reader to an async writer.
///
/// This is wrapper for `tokio::io::copy` with error context
#[cfg(feature = "coroutine")]
#[inline(always)]
pub async fn co_copy<'a, R, W>(reader: &'a mut R, writer: &'a mut W) -> crate::Result<u64>
where
    R: tokio::io::AsyncRead + Unpin + ?Sized,
    W: tokio::io::AsyncWrite + Unpin + ?Sized,
{
    tokio::io::copy(reader, writer)
        .await
        .context("async stream copy failed")
}
