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
#[inline(always)]
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> crate::Result<u64> {
    copy_impl(from.as_ref(), to.as_ref())
}
fn copy_impl(from: &Path, to: &Path) -> crate::Result<u64> {
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
#[inline(always)]
pub fn get_mtime(path: impl AsRef<Path>) -> crate::Result<Option<Time>> {
    get_mtime_impl(path.as_ref())
}
fn get_mtime_impl(path: &Path) -> crate::Result<Option<Time>> {
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
#[inline(always)]
pub fn set_mtime(path: impl AsRef<Path>, time: Time) -> crate::Result<()> {
    set_mtime_impl(path.as_ref(), time)
}
fn set_mtime_impl(path: &Path, time: Time) -> crate::Result<()> {
    crate::check!(
        filetime::set_file_mtime(path, time),
        "failed to set modification time for '{}'",
        path.display()
    )
}

/// Remove `path` as either a file or empty directory.
///
/// No-op if the path does not exist.
/// Error if the path is a non-empty directory.
#[inline(always)]
pub fn remove(path: impl AsRef<Path>) -> crate::Result<()> {
    remove_impl(path.as_ref())
}
fn remove_impl(path: &Path) -> crate::Result<()> {
    if !path.exists() {
        crate::trace!("remove: is absent: '{}'", path.display());
        return Ok(());
    }
    crate::trace!("remove '{}'", path.display());
    if path.is_dir() {
        return crate::check!(
            std::fs::remove_dir(path),
            "failed to remove directory '{}'",
            path.display()
        );
    }
    crate::check!(
        std::fs::remove_file(path),
        "failed to remove file '{}'",
        path.display()
    )
}

/// Async version of [`remove`]
#[cfg(feature = "coroutine")]
#[inline(always)]
pub async fn co_remove(path: impl AsRef<Path>) -> crate::Result<()> {
    co_remove_impl(path.as_ref()).await
}
#[cfg(feature = "coroutine")]
async fn co_remove_impl(path: &Path) -> crate::Result<()> {
    if !path.exists() {
        crate::trace!("co_remove: is absent: '{}'", path.display());
        return Ok(());
    }
    crate::trace!("co_remove '{}'", path.display());
    if path.is_dir() {
        return crate::check!(
            tokio::fs::remove_dir(path).await,
            "failed to remove directory '{}'",
            path.display()
        );
    }
    crate::check!(
        tokio::fs::remove_file(path).await,
        "failed to remove file '{}'",
        path.display()
    )
}

/// Move a file or directory.
///
/// Unlike `std::fs::rename`, this will fallback to less efficient methods
/// of moving if the paths are on separate file systems. Currently, renaming
/// directories on separate filesystems is not supported to prevent unintended
/// behavior.
///
/// # If `from` is file
/// - will error if `to` is a directory
/// - `to` will be replaced if exists
///
/// # If `from` is a directory
/// - will error if `to` exists and is not a directory or is a non-empty directory (use `cu::fs::rec_remove(to)` to delete
///   it first), or is on a different filesystem than `from`.
#[inline(always)]
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> crate::Result<()> {
    rename_impl(from.as_ref(), to.as_ref())
}
#[crate::context("failed to rename '{}' to '{}'", from.display(), to.display())]
fn rename_impl(from: &Path, to: &Path) -> crate::Result<()> {
    crate::trace!("rename: '{}' to '{}'", from.display(), to.display());
    let Ok(from_meta) = from.metadata() else {
        crate::bail!("source does not exist");
    };
    if from_meta.is_file() {
        if to.is_dir() {
            crate::bail!("target is a directory");
        }
        let Err(e) = std::fs::rename(from, to) else {
            return Ok(());
        };
        crate::trace!("rename failed: {e}, trying fallback");
        // fallback is copy and delete old
        crate::fs::copy(from, to)?;
        crate::fs::remove(from)?;
        return Ok(());
    }
    match to.metadata().ok() {
        None => {
            // does not exist, OK
        }
        Some(to_meta) => {
            if !to_meta.is_dir() {
                // exists and is not directory
                crate::bail!("target is not a directory");
            }
            if !crate::fs::is_empty_dir(to)? {
                crate::bail!("target is a non-empty directory");
            }
            // remove old
            crate::fs::remove(to)?;
        }
    }
    // no fallback if std rename fails
    std::fs::rename(from, to)?;
    Ok(())
}
