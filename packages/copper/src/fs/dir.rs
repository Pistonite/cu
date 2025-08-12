use std::path::Path;

use crate::pre::*;

/// Check if a directory is empty. Will error if cannot read the directory
pub fn is_empty(path: impl AsRef<Path>) -> crate::Result<bool> {
    let path = path.as_ref();
    crate::trace!("is_empty '{}'", path.display());
    let mut x = read_dir(path)?;
    Ok(x.next().is_none())
}

/// Ensure `path` exists and is a directory, creating it and all parent directories
/// if not.
pub fn make_dir(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    let exists = path.exists();
    crate::trace!("make_dir '{}'", path.display());
    if exists && !path.is_dir() {
        crate::bail!(
            "{} exists and is not a directory or not accessible",
            path.display()
        );
    }
    if !exists {
        crate::trace!("make_dir: creating '{}'", path.display());
        crate::check!(
            std::fs::create_dir_all(path),
            "failed to create directory '{}'",
            path.display()
        )?;
    } else {
        crate::trace!("make_dir: exists '{}'", path.display());
    }
    Ok(())
}

/// Async version of [`make_dir`]
#[cfg(feature = "coroutine")]
pub async fn co_make_dir(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    let exists = path.exists();
    crate::trace!("co_make_dir '{}'", path.display());
    if exists && !path.is_dir() {
        crate::bail!(
            "{} exists and is not a directory or not accessible",
            path.display()
        );
    }
    if !exists {
        crate::trace!("co_make_dir: creating '{}'", path.display());
        crate::check!(
            tokio::fs::create_dir_all(path).await,
            "failed to create directory '{}'",
            path.display()
        )?;
    } else {
        crate::trace!("co_make_dir: exists '{}'", path.display());
    }
    Ok(())
}

/// Ensure `path` exists and is an empty directory.
///
/// If `path` does not exist, it will be created.
/// Current contents in `path` will be removed.
pub fn make_dir_empty(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    make_dir(path)?;
    remove_contents(path)
}

/// Async version of [`make_dir_empty`]
#[cfg(feature = "coroutine")]
pub async fn co_make_dir_empty(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    co_make_dir(path).await?;
    co_remove_contents(path).await
}

/// Ensure `path` either does not exist, or is an empty directory.
pub fn make_dir_absent_or_empty(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }
    remove_contents(path)
}
/// Async version of [`make_dir_absent_or_empty`]
#[cfg(feature = "coroutine")]
pub async fn co_make_dir_absent_or_empty(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }
    co_remove_contents(path).await
}

/// Remove `path` as either a file or empty directory.
///
/// No-op if the path does not exist.
/// Error if the path is a non-empty directory.
pub fn remove(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
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
pub async fn co_remove(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
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

/// Recursively remove `path` and all of its contents.
///
/// No-op if the path does not exist.
/// Error if the path is a file or a link.
/// Does not follow symlinks.
pub fn rec_remove(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        crate::trace!("rec_remove: is absent: '{}'", path.display());
        return Ok(());
    }
    crate::trace!("rec_remove '{}'", path.display());
    if !path.is_dir() {
        crate::bail!("'{}' is not a directory", path.display());
    }
    crate::check!(
        std::fs::remove_dir_all(path),
        "failed to recursively remove '{}'",
        path.display()
    )
}

/// Async version of [`rec_remove`]
#[cfg(feature = "coroutine")]
pub async fn co_rec_remove(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        crate::trace!("co_rec_remove: is absent: '{}'", path.display());
        return Ok(());
    }
    crate::trace!("co_rec_remove '{}'", path.display());
    if !path.is_dir() {
        crate::bail!("'{}' is not a directory", path.display());
    }
    crate::check!(
        tokio::fs::remove_dir_all(path).await,
        "failed to recursively remove '{}'",
        path.display()
    )
}

/// Remove all of `path`'s contents, but does not remove itself.
///
/// Error if the path is not a directory. Does not follow symlinks.
/// If any of the directory content fails to read, it will propagate
/// the error.
pub fn remove_contents(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    crate::trace!("remove_contents '{}'", path.display());
    if !path.is_dir() {
        if !path.exists() {
            crate::bail!("'{}' does not exist", path.display());
        }
        crate::bail!("'{}' is not a directory", path.display());
    }
    for entry in read_dir(path)? {
        let entry = crate::check!(entry, "failed to read entry inside '{}'", path.display())?;
        let entry_path = entry.path();
        let file_type = crate::check!(
            entry.file_type(),
            "failed to read entry type for '{}'",
            entry_path.display()
        )?;
        if file_type.is_dir() {
            rec_remove(entry_path)?;
        } else {
            remove(entry_path)?;
        }
    }
    Ok(())
}

/// Async version of [`remove_contents`]. Note that this is not fail-fast.
/// If some entry fails to delete, all entries will still be attempted to be deleted.
#[cfg(feature = "coroutine")]
pub async fn co_remove_contents(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    crate::trace!("remove_contents '{}'", path.display());
    if !path.is_dir() {
        if !path.exists() {
            crate::bail!("'{}' does not exist", path.display());
        }
        crate::bail!("'{}' is not a directory", path.display());
    }
    let mut x = co_read_dir(path).await?;
    // using a join set to take full advantage of blocking thread pool
    let mut join_set = tokio::task::JoinSet::new();
    loop {
        let entry = x.next_entry().await;
        let entry = crate::check!(entry, "failed to read entry inside '{}'", path.display())?;
        let Some(entry) = entry else {
            break;
        };
        let entry_path = entry.path();
        let file_type = crate::check!(
            entry.file_type().await,
            "failed to read entry type for '{}'",
            entry_path.display()
        )?;
        if file_type.is_dir() {
            join_set.spawn(async move { co_rec_remove(entry_path).await });
        } else {
            join_set.spawn(async move { co_remove(entry_path).await });
        }
    }
    let mut has_failure = false;
    while let Some(x) = join_set.join_next().await {
        match x {
            Err(e) => {
                crate::debug!("failed to remove some entry in '{}': {e}", path.display());
                has_failure = true;
            }
            Ok(Err(e)) => {
                crate::debug!("failed to remove some entry in '{}': {e}", path.display());
                has_failure = true;
            }
            Ok(Ok(())) => {}
        }
    }
    if has_failure {
        crate::bail!(
            "failed to remove one or more entries in '{}'",
            path.display()
        );
    }
    Ok(())
}

pub type ReadDir = std::fs::ReadDir;
#[cfg(feature = "coroutine")]
pub type CoReadDir = tokio::fs::ReadDir;
pub type DirEntry = std::fs::DirEntry;
#[cfg(feature = "coroutine")]
pub type CoDirEntry = tokio::fs::DirEntry;
/// `std::fs::read_dir` with error reporting
pub fn read_dir(path: impl AsRef<Path>) -> crate::Result<ReadDir> {
    let path = path.as_ref();
    crate::check!(
        std::fs::read_dir(path),
        "cannot read directory '{}'",
        path.display()
    )
}

/// Async version of [`read_dir`]
#[cfg(feature = "coroutine")]
pub async fn co_read_dir(path: impl AsRef<Path>) -> crate::Result<CoReadDir> {
    let path = path.as_ref();
    crate::check!(
        tokio::fs::read_dir(path).await,
        "cannot read directory '{}'",
        path.display()
    )
}
