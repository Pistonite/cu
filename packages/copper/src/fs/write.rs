use std::path::Path;

use crate::pre::*;
/// Writes the content to the path, automatically creating the directory if doesn't exist.
#[inline(always)]
pub fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> crate::Result<()> {
    write_impl(path.as_ref(), content.as_ref())
}
fn write_impl(path: &Path, content: &[u8]) -> crate::Result<()> {
    crate::trace!("write '{}'", path.display());
    let Err(x) = std::fs::write(path, content) else {
        return Ok(());
    };
    if x.kind() == std::io::ErrorKind::NotFound {
        // ensure parent dir exists and try again
        if let Ok(parent) = path.parent_abs() {
            crate::trace!("retrying with parent creation: write '{}'", path.display());
            if !parent.exists() {
                crate::check!(
                    super::make_dir(&parent),
                    "could not automatically create parent directory for '{}'",
                    path.display()
                )?;
                // try again
                return crate::check!(
                    std::fs::write(path, content),
                    "failed to write to file '{}'",
                    path.display()
                );
            }
        }
    }
    make_write_error(path, x)
}

/// Make a non-buffered writer to the file
///
/// Note that using a writer only really makes a difference
/// if the content is large (i.e. does not fit in memory)
///
/// If you are making small writes, use [`cu::fs::buf_writer`]
///
/// Make sure to manually flush to capture errors while flushing
///
/// [`cu::fs::buf_writer`]: buf_writer
#[inline(always)]
pub fn writer(path: impl AsRef<Path>) -> crate::Result<std::fs::File> {
    writer_impl(path.as_ref())
}
fn writer_impl(path: &Path) -> crate::Result<std::fs::File> {
    crate::trace!("writer '{}'", path.display());
    if let Ok(file) = std::fs::File::create(path) {
        return Ok(file);
    }
    // write empty to the file to ensure it exists
    write(path, [])?;
    crate::trace!(
        "retrying after creating the file: writer '{}'",
        path.display()
    );
    // try again
    match std::fs::File::create(path) {
        Ok(file) => Ok(file),
        Err(e) => make_write_error(path, e),
    }
}

pub type BufFile = std::io::BufWriter<std::fs::File>;

/// Make a buffered writer to the file
///
/// Note that using a writer only really makes a difference
/// if the content is large (i.e. does not fit in memory)
///
/// If you know the writes will be in large chunks anyway, use unbuffered [`cu::fs::writer`]
///
/// Make sure to manually flush to capture errors while flushing
///
/// [`cu::fs::writer`]: writer
#[inline(always)]
pub fn buf_writer(path: impl AsRef<Path>) -> crate::Result<BufFile> {
    writer(path).map(BufFile::new)
}

/// Async version of [`write`](function@crate::fs::write)
#[cfg(feature = "coroutine")]
#[inline(always)]
pub async fn co_write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> crate::Result<()> {
    co_write_impl(path.as_ref(), content.as_ref()).await
}
#[cfg(feature = "coroutine")]
async fn co_write_impl(path: &Path, content: &[u8]) -> crate::Result<()> {
    crate::trace!("co_write '{}'", path.display());
    let Err(x) = tokio::fs::write(path, content).await else {
        return Ok(());
    };
    if x.kind() == std::io::ErrorKind::NotFound {
        // ensure parent dir exists and try again
        if let Ok(parent) = path.parent_abs() {
            crate::trace!(
                "retrying with parent creation: co_write '{}'",
                path.display()
            );
            if !parent.exists() {
                crate::check!(
                    super::co_make_dir(&parent).await,
                    "could not automatically create parent directory for '{}'",
                    path.display()
                )?;
                // try again
                return crate::check!(
                    tokio::fs::write(path, content).await,
                    "failed to write to file '{}'",
                    path.display()
                );
            }
        }
    }
    make_write_error(path, x)
}

fn make_write_error<T>(path: &Path, e: std::io::Error) -> crate::Result<T> {
    if e.kind() == std::io::ErrorKind::NotADirectory {
        if let Ok(parent) = path.parent_abs() {
            if parent.exists() && !parent.is_dir() {
                return Err(e).context(format!(
                    "the parent path '{}' exists and is not a directory",
                    path.display()
                ))?;
            }
        }
    }
    Err(e).context(format!("failed to write to file '{}'", path.display()))
}

/// Serialize data as JSON and write it to a file.
#[cfg(feature = "json")]
#[inline(always)]
pub fn write_json<S: serde::Serialize>(path: impl AsRef<Path>, content: &S) -> crate::Result<()> {
    write_json_impl(path.as_ref(), crate::json::stringify(content))
}
/// Serialize data as JSON, prettified, and write it to a file.
#[cfg(feature = "json")]
#[inline(always)]
pub fn write_json_pretty<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    write_json_impl(path.as_ref(), crate::json::stringify_pretty(content))
}
#[cfg(feature = "json")]
fn write_json_impl(path: &Path, content: crate::Result<String>) -> crate::Result<()> {
    let content = crate::check!(content, "failed to write json to file '{}'", path.display())?;
    write(path, content)
}

/// Async version of [`write_json`]. The serialization part is synchronous, since it does not
/// involve IO.
#[cfg(all(feature = "json", feature = "coroutine"))]
#[inline(always)]
pub async fn co_write_json<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    co_write_json_impl(path.as_ref(), crate::json::stringify(content)).await
}
/// Async version of [`write_json_pretty`]. The serialization part is synchronous, since it does not
/// involve IO.
#[cfg(all(feature = "json", feature = "coroutine"))]
pub async fn co_write_json_pretty<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    co_write_json_impl(path.as_ref(), crate::json::stringify_pretty(content)).await
}
#[cfg(all(feature = "json", feature = "coroutine"))]
async fn co_write_json_impl(path: &Path, content: crate::Result<String>) -> crate::Result<()> {
    let content = crate::check!(content, "failed to write json to file '{}'", path.display())?;
    co_write(path, content).await
}

/// Serialize data as TOML and write it to a file.
#[cfg(feature = "toml")]
#[inline(always)]
pub fn write_toml<S: serde::Serialize>(path: impl AsRef<Path>, content: &S) -> crate::Result<()> {
    write_toml_impl(path.as_ref(), crate::toml::stringify(content))
}
/// Serialize data as TOML, prettified, and write it to a file.
#[cfg(feature = "toml")]
#[inline(always)]
pub fn write_toml_pretty<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    write_toml_impl(path.as_ref(), crate::toml::stringify_pretty(content))
}
#[cfg(feature = "toml")]
fn write_toml_impl(path: &Path, content: crate::Result<String>) -> crate::Result<()> {
    let content = crate::check!(content, "failed to write toml to file '{}'", path.display())?;
    write(path, content)
}

/// Async version of [`write_toml`]. The serialization part is synchronous, since it does not
/// involve IO.
#[cfg(all(feature = "toml", feature = "coroutine"))]
#[inline(always)]
pub async fn co_write_toml<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    co_write_toml_impl(path.as_ref(), crate::toml::stringify(content)).await
}
/// Async version of [`write_toml_pretty`]. The serialization part is synchronous, since it does not
/// involve IO.
#[cfg(all(feature = "toml", feature = "coroutine"))]
pub async fn co_write_toml_pretty<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    co_write_toml_impl(path.as_ref(), crate::toml::stringify_pretty(content)).await
}
#[cfg(all(feature = "toml", feature = "coroutine"))]
async fn co_write_toml_impl(path: &Path, content: crate::Result<String>) -> crate::Result<()> {
    let content = crate::check!(content, "failed to write toml to file '{}'", path.display())?;
    co_write(path, content).await
}

/// Serialize data as YAML and write it to a file.
#[cfg(feature = "yaml")]
#[inline(always)]
pub fn write_yaml<S: serde::Serialize>(path: impl AsRef<Path>, content: &S) -> crate::Result<()> {
    write_yaml_impl(path.as_ref(), crate::yaml::stringify(content))
}
#[cfg(feature = "yaml")]
fn write_yaml_impl(path: &Path, content: crate::Result<String>) -> crate::Result<()> {
    let content = crate::check!(content, "failed to write yaml to file '{}'", path.display())?;
    write(path, content)
}

/// Async version of [`write_yaml`]. The serialization part is synchronous, since it does not
/// involve IO.
#[cfg(all(feature = "yaml", feature = "coroutine"))]
#[inline(always)]
pub async fn co_write_yaml<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    co_write_yaml_impl(path.as_ref(), crate::yaml::stringify(content)).await
}
#[cfg(all(feature = "yaml", feature = "coroutine"))]
async fn co_write_yaml_impl(path: &Path, content: crate::Result<String>) -> crate::Result<()> {
    let content = crate::check!(content, "failed to write yaml to file '{}'", path.display())?;
    co_write(path, content).await
}
