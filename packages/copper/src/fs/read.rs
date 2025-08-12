use std::path::Path;

use crate::pre::*;

/// `std::fs::read` with tracing and error reporting
pub fn read(path: impl AsRef<Path>) -> crate::Result<Vec<u8>> {
    let path = path.as_ref();
    crate::trace!("read '{}'", path.display());
    crate::check!(
        std::fs::read(path),
        "failed to read file '{}' as bytes",
        path.display()
    )
}

/// Async version of [`read`]
#[cfg(feature = "coroutine")]
pub async fn co_read(path: impl AsRef<Path>) -> crate::Result<Vec<u8>> {
    let path = path.as_ref();
    crate::trace!("co_read '{}'", path.display());
    crate::check!(
        tokio::fs::read(path).await,
        "failed to read file '{}' as bytes",
        path.display()
    )
}

/// `std::fs::read_to_string` with tracing and error reporting
pub fn read_string(path: impl AsRef<Path>) -> crate::Result<String> {
    let path = path.as_ref();
    crate::trace!("read_string '{}'", path.display());
    crate::check!(
        std::fs::read_to_string(path),
        "failed to read file '{}' as string",
        path.display()
    )
}

/// Async version of [`read_string`]
#[cfg(feature = "coroutine")]
pub async fn co_read_string(path: impl AsRef<Path>) -> crate::Result<String> {
    let path = path.as_ref();
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
pub fn reader(path: impl AsRef<Path>) -> crate::Result<Reader> {
    let path = path.as_ref();
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
#[cfg(feature = "coroutine")]
pub async fn co_reader(path: impl AsRef<Path>) -> crate::Result<CoReader> {
    let path = path.as_ref();
    crate::trace!("co_reader '{}'", path.display());
    let file = crate::check!(
        tokio::fs::File::open(path).await,
        "failed to open file '{}'",
        path.display()
    )?;
    Ok(tokio::io::BufReader::new(file))
}
