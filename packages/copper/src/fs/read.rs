use std::path::Path;

use crate::pre::*;

/// `std::fs::read` with tracing and error reporting
#[inline(always)]
pub fn read(path: impl AsRef<Path>) -> crate::Result<Vec<u8>> {
    read_impl(path.as_ref())
}
fn read_impl(path: &Path) -> crate::Result<Vec<u8>> {
    crate::trace!("read '{}'", path.display());
    crate::check!(
        std::fs::read(path),
        "failed to read file '{}' as bytes",
        path.display()
    )
}

/// Async version of [`read`]
#[cfg(feature = "coroutine")]
#[inline(always)]
pub async fn co_read(path: impl AsRef<Path>) -> crate::Result<Vec<u8>> {
    co_read_impl(path.as_ref()).await
}
#[cfg(feature = "coroutine")]
async fn co_read_impl(path: &Path) -> crate::Result<Vec<u8>> {
    crate::trace!("co_read '{}'", path.display());
    crate::check!(
        tokio::fs::read(path).await,
        "failed to read file '{}' as bytes",
        path.display()
    )
}

/// `std::fs::read_to_string` with tracing and error reporting
#[inline(always)]
pub fn read_string(path: impl AsRef<Path>) -> crate::Result<String> {
    read_string_impl(path.as_ref())
}
fn read_string_impl(path: &Path) -> crate::Result<String> {
    crate::trace!("read_string '{}'", path.display());
    crate::check!(
        std::fs::read_to_string(path),
        "failed to read file '{}' as string",
        path.display()
    )
}

/// Async version of [`read_string`]
#[cfg(feature = "coroutine")]
#[inline(always)]
pub async fn co_read_string(path: impl AsRef<Path>) -> crate::Result<String> {
    co_read_string_impl(path.as_ref()).await
}
#[cfg(feature = "coroutine")]
async fn co_read_string_impl(path: &Path) -> crate::Result<String> {
    crate::trace!("co_read_string '{}'", path.display());
    crate::check!(
        tokio::fs::read_to_string(path).await,
        "failed to read file '{}' as string",
        path.display()
    )
}

/// A buffered file reader
pub type Reader = std::io::BufReader<std::fs::File>;

/// Open a file for buffered reading
///
/// Note that using a reader only really makes a difference
/// if the file is large (i.e. does not fit in memory)
#[inline(always)]
pub fn reader(path: impl AsRef<Path>) -> crate::Result<Reader> {
    reader_impl(path.as_ref())
}
fn reader_impl(path: &Path) -> crate::Result<Reader> {
    crate::trace!("reader '{}'", path.display());
    let file = crate::check!(
        std::fs::File::open(path),
        "failed to open file '{}'",
        path.display()
    )?;
    Ok(std::io::BufReader::new(file))
}

/// A buffered async file reader
#[cfg(feature = "coroutine")]
pub type CoReader = tokio::io::BufReader<tokio::fs::File>;

/// Open a file for buffered asynchronous reading
///
/// Note that using a reader only really makes a difference
/// if the file is large (i.e. does not fit in memory)
#[cfg(feature = "coroutine")]
#[inline(always)]
pub async fn co_reader(path: impl AsRef<Path>) -> crate::Result<CoReader> {
    co_reader_impl(path.as_ref()).await
}
#[cfg(feature = "coroutine")]
async fn co_reader_impl(path: &Path) -> crate::Result<CoReader> {
    crate::trace!("co_reader '{}'", path.display());
    let file = crate::check!(
        tokio::fs::File::open(path).await,
        "failed to open file '{}'",
        path.display()
    )?;
    Ok(tokio::io::BufReader::new(file))
}
