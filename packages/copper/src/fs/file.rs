use std::path::{Path, PathBuf};

use super::Time;

use crate::pre::*;

/// `std::env::current_exe()` with error reporting
///
/// ```rust
/// # use pistonite_cu as cu;
/// # fn main() -> cu::Result<()> {
/// let path = cu::fs::current_exe()?;
/// assert!(path.is_absolute());
/// # Ok(()) }
/// ```
pub fn current_exe() -> crate::Result<PathBuf> {
    std::env::current_exe().context("failed to get current exe path")
}

/// Copy a file from one to another.
///
/// If copy failed, it will attempt to fallback to using read and write.
/// Directories will be created for the target location if not already exists.
///
/// if from and to is the pointing to the same, it might be truncated.
///
/// The number of bytes in `to` is returned.
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> crate::Result<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    crate::trace!("copy from='{}' to='{}'", from.display(), to.display());
    if to.is_dir() {
        crate::bail!(
            "cannot copy from '{}' to '{}': target is directory",
            from.display(),
            to.display()
        );
    }
    let copy_error = match std::fs::copy(from, to) {
        Ok(v) => return Ok(v),
        Err(e) => e,
    };
    // we know the fallback will also fail in these cases
    if !from.exists() {
        crate::rethrow!(
            copy_error,
            "cannot copy from '{}' to '{}': source does not exist",
            from.display(),
            to.display()
        );
    }
    if from.is_dir() {
        crate::rethrow!(
            copy_error,
            "cannot copy from '{}' to '{}': source is directory",
            from.display(),
            to.display()
        );
    }
    // try the fallback
    crate::trace!(
        "copy failed, attempting fallback, from='{}' to='{}'",
        from.display(),
        to.display()
    );
    let bytes = match super::read(from) {
        Err(e) => {
            crate::trace!(
                "fallback copy failed when reading '{}': {e:?}",
                from.display()
            );
            crate::rethrow!(
                copy_error,
                "failed to copy file from '{}' to '{}'",
                from.display(),
                to.display()
            );
        }
        Ok(x) => x,
    };
    let size = bytes.len() as u64;
    match super::write(to, bytes) {
        Err(e) => {
            crate::trace!(
                "fallback copy failed when writing '{}': {e:?}",
                to.display()
            );
            crate::rethrow!(
                copy_error,
                "failed to copy file from '{}' to '{}'",
                from.display(),
                to.display()
            );
        }
        Ok(_) => Ok(size),
    }
}

/// Get the modified time for a file.
///
/// If the file doesn't exist, None is returned
pub fn get_mtime(path: impl AsRef<Path>) -> crate::Result<Option<Time>> {
    let path = path.as_ref();
    match path.metadata() {
        Ok(meta) => Ok(Some(Time::from_last_modification_time(&meta))),
        Err(e) => {
            if !path.exists() {
                return Ok(None);
            }
            crate::rethrow!(
                e,
                "failed to get modification time for '{}'",
                path.display()
            );
        }
    }
}

/// Set the modified time for a file
pub fn set_mtime(path: impl AsRef<Path>, time: Time) -> crate::Result<()> {
    let path = path.as_ref();
    crate::check!(
        filetime::set_file_mtime(path, time),
        "failed to set modification time for '{}'",
        path.display()
    )
}
