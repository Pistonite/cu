use std::{ffi::OsString, fs::FileType, path::{Path, PathBuf}};

use crate::{Context, PathExtension};

/// `std::fs::read` with tracing and error reporting
pub fn read(path: impl AsRef<Path>) -> crate::Result<Vec<u8>> {
    let path = path.as_ref();
    crate::trace!("read '{}'", path.display());
    std::fs::read(path)
        .with_context(|| format!("failed to read file '{}' as bytes", path.display()))
}

/// `tokio::fs::read` with tracing and error reporting
#[cfg(feature = "coroutine")]
pub async fn co_read(path: impl AsRef<Path>) -> crate::Result<Vec<u8>> {
    let path = path.as_ref();
    crate::trace!("co_read '{}'", path.display());
    tokio::fs::read(path).await
        .with_context(|| format!("failed to read file '{}' as bytes", path.display()))
}

/// `std::fs::read_to_string` with tracing and error reporting
pub fn read_string(path: impl AsRef<Path>) -> crate::Result<String> {
    let path = path.as_ref();
    crate::trace!("read_string '{}'", path.display());
    std::fs::read_to_string(path)
        .with_context(|| format!("failed to read file '{}' as string", path.display()))
}

/// `tokio::fs::read_to_string` with tracing and error reporting
#[cfg(feature = "coroutine")]
pub async fn co_read_string(path: impl AsRef<Path>) -> crate::Result<String> {
    let path = path.as_ref();
    crate::trace!("co_read_string '{}'", path.display());
    tokio::fs::read_to_string(path).await
        .with_context(|| format!("failed to read file '{}' as string", path.display()))
}

/// A buffered file reader
pub type Reader = std::io::BufReader<std::fs::File>;

/// Open a file for buffered reading
pub fn reader(path: impl AsRef<Path>) -> crate::Result<Reader> {
    let path = path.as_ref();
    crate::trace!("reader '{}'", path.display());
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open file '{}'", path.display()))?;
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
    let file = tokio::fs::File::open(path).await
        .with_context(|| format!("failed to open file '{}'", path.display()))?;
    Ok(tokio::io::BufReader::new(file))
}

/// Writes the content to the path, automatically creating the directory if doesn't exist.
pub fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> crate::Result<()> {
    let path = path.as_ref();
    let content = content.as_ref();
    crate::trace!("write '{}'", path.display());
    let Err(x) = std::fs::write(path, content) else {
        return Ok(());
    };
    if x.kind() == std::io::ErrorKind::NotFound {
        // ensure parent dir exists and try again
        if let Ok(parent) = path.parent_abs() {
            crate::trace!("retrying with parent creation: write '{}'", path.display());
            if !parent.exists() {
                ensure_dir(&parent).with_context(|| {
                    format!(
                        "could not automatically create parent directory '{}'",
                        parent.display()
                    )
                })?;
                // try again
                std::fs::write(path, content)
                    .with_context(|| format!("failed to write to file '{}'", path.display()))?;
                return Ok(());
            }
        }
    }
    if x.kind() == std::io::ErrorKind::NotADirectory {
        if let Ok(parent) = path.parent_abs() {
            if parent.exists() && !parent.is_dir() {
                return Err(x).context(format!(
                    "the parent path '{}' exists and is not a directory",
                    path.display()
                ))?;
            }
        }
    }
    Err(x).context(format!("failed to write to {}", path.display()))
}

/// Async version of [`write`]
#[cfg(feature = "coroutine")]
pub async fn co_write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> crate::Result<()> {
    let path = path.as_ref();
    let content = content.as_ref();
    crate::trace!("co_write '{}'", path.display());
    let Err(x) = tokio::fs::write(path, content).await else {
        return Ok(());
    };
    if x.kind() == std::io::ErrorKind::NotFound {
        // ensure parent dir exists and try again
        if let Ok(parent) = path.parent_abs() {
            crate::trace!("retrying with parent creation: co_write '{}'", path.display());
            if !parent.exists() {
                ensure_dir(&parent).with_context(|| {
                    format!(
                        "could not automatically create parent directory '{}'",
                        parent.display()
                    )
                })?;
                // try again
                tokio::fs::write(path, content).await
                    .with_context(|| format!("failed to write to file '{}'", path.display()))?;
                return Ok(());
            }
        }
    }
    if x.kind() == std::io::ErrorKind::NotADirectory {
        if let Ok(parent) = path.parent_abs() {
            if parent.exists() && !parent.is_dir() {
                return Err(x).context(format!(
                    "the parent path '{}' exists and is not a directory",
                    path.display()
                ))?;
            }
        }
    }
    Err(x).context(format!("failed to write to {}", path.display()))
}

/// Serialize data as JSON and write it to a file.
#[cfg(feature = "json")]
pub fn write_json(path: impl AsRef<Path>, content: impl serde::Serialize) -> crate::Result<()> {
    todo!()
}

/// Ensure `path` exists and is a directory, creating one if not
pub fn ensure(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    let exists = path.exists();
    crate::trace!("ensure_dir '{}'", path.display());
    if exists && !path.is_dir() {
        crate::bail!(
            "{} exists and is not a directory or not accessible",
            path.display()
        );
    }
    if !exists {
        crate::trace!("ensure_dir: creating '{}'", path.display());
        std::fs::create_dir_all(path)
            .with_context(|| format!("failed to create directory at {}", path.display()))?
    } else {
        crate::trace!("ensure_dir: exists '{}'", path.display());
    }
    if !path.exists() || !path.is_dir() {
        crate::bail!("directory creation failed for: {}", path.display());
    }
    Ok(())
}

/// Asynchronous version of [`ensure`]
pub async fn co_ensure(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    let exists = path.exists();
    crate::trace!("co_ensure_dir '{}'", path.display());
    if exists && !path.is_dir() {
        crate::bail!(
            "{} exists and is not a directory or not accessible",
            path.display()
        );
    }
    if !exists {
        crate::trace!("co_ensure_dir: creating '{}'", path.display());
        tokio::fs::create_dir_all(path).await
            .with_context(|| format!("failed to create directory at {}", path.display()))?
    } else {
        crate::trace!("co_ensure_dir: exists '{}'", path.display());
    }
    if !path.exists() || !path.is_dir() {
        crate::bail!("directory creation failed for: {}", path.display());
    }
    Ok(())
}

/// Ensure `path` exists and is an empty directory, creating one if not
pub fn ensure_empty(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    crate::trace!("ensure_empty '{}'", path.display());
    ensure(path)?;
    let exists = path.exists();
    crate::trace!("ensure_dir '{}'", path.display());
    if exists && !path.is_dir() {
        crate::bail!(
            "{} exists and is not a directory or not accessible",
            path.display()
        );
    }
    if !exists {
        crate::trace!("ensure_dir: creating '{}'", path.display());
        std::fs::create_dir_all(path)
            .with_context(|| format!("failed to create directory at {}", path.display()))?
    } else {
        crate::trace!("ensure_dir: exists '{}'", path.display());
    }
    if !path.exists() || !path.is_dir() {
        crate::bail!("directory creation failed for: {}", path.display());
    }
    Ok(())
}

/// Check if a directory is empty. Will error if doesn't exist
pub fn is_empty(path: impl AsRef<Path>) -> crate::Result<bool> {
    let path = path.as_ref();
    crate::trace!("is_empty '{}'", path.display());
    let mut x = crate::check!(
        std::fs::read_dir(path), 
        "cannot read directory '{}'",
        path.display()
    );
    Ok(x.next().is_none())
}


pub fn walk_with(path: impl AsRef<Path>, 
    filter: impl for<'a> Fn(crate::Result<&WalkEntry<'a>>) -> crate::Result<bool>) -> crate::Result<()> {

    let path = path.as_ref();
    crate::trace!("walk '{}'", path.display());
    let mut reader = crate::check!(
        std::fs::read_dir(path), 
        "cannot read directory '{}'",
        path.display()
    );



}
#[doc(hidden)]
pub type WalkEntry_<'a, Entry> = (FileType, &'a Path, OsString, Entry);
pub type WalkEntry<'a> = WalkEntry_<'a, std::fs::DirEntry>;
pub type CoWalkEntry<'a> = WalkEntry_<'a, tokio::fs::DirEntry>;

pub struct Walk {
    /// The path of the containing directory
    /// of the current entry, relative from the root of the walk
    containing_rel: PathBuf,
    stack: Vec<std::fs::ReadDir>
}

impl Walk {
    pub fn iter(&mut self) -> WalkIter<'_> {
        WalkIter { walk: self }
    }
}
pub struct WalkIter<'a> {
    walk: &'a mut Walk
}
impl<'a> Iterator for WalkIter<'a> {
    type Item=WalkEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

/// Remove `path` as a directory recursively. No-op if the path does not exist.
pub fn remove_dir(path: impl AsRef<Path>) -> crate::Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(path)
        .with_context(|| format!("failed to remove '{}' recursively", path.display()))
}

/// Get the path to the current executable.
pub fn current_exe() -> crate::Result<PathBuf> {
    std::env::current_exe().context("failed to get current exe path")
}
