//! Directory-only operations (Will error if path is a file or link)
use std::collections::BTreeSet;
use std::path::Path;

use crate::pre::*;

/// Check if a directory exists and is empty.
#[inline(always)]
pub fn is_empty_dir(path: impl AsRef<Path>) -> crate::Result<bool> {
    is_empty_dir_impl(path.as_ref())
}
fn is_empty_dir_impl(path: &Path) -> crate::Result<bool> {
    crate::trace!("is_empty_dir '{}'", path.display());
    let mut x = read_dir_impl(path)?;
    Ok(x.next().is_none())
}

/// Ensure `path` exists and is a directory, creating it and all parent directories
/// if not.
#[inline(always)]
pub fn make_dir(path: impl AsRef<Path>) -> crate::Result<()> {
    make_dir_impl(path.as_ref())
}
fn make_dir_impl(path: &Path) -> crate::Result<()> {
    crate::trace!("make_dir '{}'", path.display());
    let (exists, is_dir) = match std::fs::metadata(path) {
        Ok(m) => (true, m.is_dir()),
        Err(_) => (false, false),
    };
    match (exists, is_dir) {
        (true, false) => {
            crate::bail!(
                "failed to create directory: '{}' exists but is not a directory",
                path.display()
            );
        }
        (true, true) => {} // exists and is dir
        (false, _) => {
            crate::trace!("make_dir: creating '{}'", path.display());
            crate::check!(
                std::fs::create_dir_all(path),
                "failed to create directory '{}'",
                path.display()
            )?;
        }
    }
    Ok(())
}

/// Async version of [`make_dir`]
#[cfg(feature = "coroutine")]
#[inline(always)]
pub async fn co_make_dir(path: impl AsRef<Path>) -> crate::Result<()> {
    co_make_dir_impl(path.as_ref()).await
}
#[cfg(feature = "coroutine")]
async fn co_make_dir_impl(path: &Path) -> crate::Result<()> {
    crate::trace!("co_make_dir '{}'", path.display());
    let (exists, is_dir) = match std::fs::metadata(path) {
        Ok(m) => (true, m.is_dir()),
        Err(_) => (false, false),
    };
    match (exists, is_dir) {
        (true, false) => {
            crate::bail!(
                "failed to create directory: '{}' exists but is not a directory",
                path.display()
            );
        }
        (true, true) => {} // exists and is dir
        (false, _) => {
            crate::trace!("co_make_dir: creating '{}'", path.display());
            crate::check!(
                tokio::fs::create_dir_all(path).await,
                "failed to create directory '{}'",
                path.display()
            )?;
        }
    }
    Ok(())
}

/// Ensure `path` exists and is an empty directory.
///
/// If `path` does not exist, it will be created.
/// Current contents in `path` will be removed.
pub fn make_dir_empty(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    make_dir_impl(path)?;
    remove_contents_impl(path)
}

/// Async version of [`make_dir_empty`]
#[cfg(feature = "coroutine")]
pub async fn co_make_dir_empty(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    co_make_dir_impl(path).await?;
    co_remove_contents_impl(path).await
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

/// Recursively remove `path` and all of its contents.
///
/// No-op if the path does not exist.
/// Error if the path is a file or a link.
/// Does not follow symlinks.
#[inline(always)]
pub fn rec_remove(path: impl AsRef<Path>) -> crate::Result<()> {
    rec_remove_impl(path.as_ref())
}
fn rec_remove_impl(path: &Path) -> crate::Result<()> {
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
#[inline(always)]
pub async fn co_rec_remove(path: impl AsRef<Path>) -> crate::Result<()> {
    co_rec_remove_impl(path.as_ref()).await
}
#[cfg(feature = "coroutine")]
async fn co_rec_remove_impl(path: &Path) -> crate::Result<()> {
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
#[inline(always)]
pub fn remove_contents(path: impl AsRef<Path>) -> crate::Result<()> {
    remove_contents_impl(path.as_ref())
}
fn remove_contents_impl(path: &Path) -> crate::Result<()> {
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
            crate::fs::remove(entry_path)?;
        }
    }
    Ok(())
}

/// Async version of [`remove_contents`]. Note that this is not fail-fast.
/// If some entry fails to delete, all entries will still be attempted to be deleted.
#[cfg(feature = "coroutine")]
#[inline(always)]
pub async fn co_remove_contents(path: impl AsRef<Path>) -> crate::Result<()> {
    co_remove_contents_impl(path.as_ref()).await
}
#[cfg(feature = "coroutine")]
async fn co_remove_contents_impl(path: &Path) -> crate::Result<()> {
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
            join_set.spawn(async move { crate::fs::co_remove(entry_path).await });
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
#[inline(always)]
pub fn read_dir(path: impl AsRef<Path>) -> crate::Result<ReadDir> {
    read_dir_impl(path.as_ref())
}
fn read_dir_impl(path: &Path) -> crate::Result<ReadDir> {
    crate::check!(
        std::fs::read_dir(path),
        "cannot read directory '{}'",
        path.display()
    )
}

/// Async version of [`read_dir`]
#[cfg(feature = "coroutine")]
#[inline(always)]
pub async fn co_read_dir(path: impl AsRef<Path>) -> crate::Result<CoReadDir> {
    co_read_dir_impl(path.as_ref()).await
}
#[cfg(feature = "coroutine")]
async fn co_read_dir_impl(path: &Path) -> crate::Result<CoReadDir> {
    crate::check!(
        tokio::fs::read_dir(path).await,
        "cannot read directory '{}'",
        path.display()
    )
}

/// (Inefficiently) recursively copy all files in `from` directory to `to` directory.
/// Old files in `to` are NOT removed if no corresponding file exists in `from`.
/// (If that behavior is desired, call [`cu::fs::make_dir_empty`](make_dir_empty) or
/// [`cu::fs::remove_contents`](remove_contents) first)
///
/// The implementation is naive, suitable for a quick invocation in your build scripts
/// or something. Find other crates if you are looking for an efficient solution
/// that does not involve cloning the files.
///
/// # Behavior
/// - `to` will be created if does not exist.
/// - symlinks are not followed (they are copied as-is)
///
/// # Will error if:
/// - `from` is not a directory, or `to` exists and is not a directory
/// - there is a path in `to` that is a directory and not a directory in `from`, or vice versa
///
#[inline(always)]
pub fn rec_copy_inefficiently(from: impl AsRef<Path>, to: impl AsRef<Path>) -> crate::Result<()> {
    rec_copy_inefficiently_impl(from.as_ref(), to.as_ref())
}
#[crate::context("failed to copy recursively copy '{}' to '{}'", from.display(), to.display())]
fn rec_copy_inefficiently_impl(from: &Path, to: &Path) -> crate::Result<()> {
    crate::trace!(
        "rec_copy_inefficiently from='{}' to='{}'",
        from.display(),
        to.display()
    );
    if !from.is_dir() {
        crate::bail!("source does not exist or is not a directory");
    }
    let to_meta = to.metadata();
    if to_meta.is_ok_and(|x| !x.is_dir()) {
        crate::bail!("target exists and is not a directory");
    }

    // cache paths that are known to be directories in `to`
    let mut dir_cache = BTreeSet::new();
    let mut walk = crate::fs::walk(from)?;
    while let Some(entry) = walk.next() {
        let entry = entry?;
        if !entry.is_dir() {
            // ensure parent directories exists
            if !dir_cache.contains(entry.rel_containing) {
                let dir_path = to.join(entry.rel_containing);
                make_dir_impl(&dir_path)?;
                dir_cache.insert(dir_path);
            }
            // copy the file
            crate::fs::copy(
                entry.path(),
                to.join(entry.rel_containing).join(&entry.file_name),
            )?;
        } else {
            let dir_path = to.join(entry.rel_containing);
            make_dir_impl(&dir_path)?;
        }
    }

    Ok(())
}
