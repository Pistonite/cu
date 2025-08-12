use std::path::Path;

use crate::pre::*;

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

/// Async version of [`write`](function@crate::fs::write)
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

fn make_write_error(path: &Path, e: std::io::Error) -> crate::Result<()> {
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
pub fn write_json<S: serde::Serialize>(path: impl AsRef<Path>, content: &S) -> crate::Result<()> {
    let path = path.as_ref();
    let content = crate::check!(
        crate::json::stringify(content),
        "failed to write json to file '{}'",
        path.display()
    )?;
    write(path, content)
}

/// Async version of [`write_json`]. The serialization part is synchronous, since it does not
/// involve IO.
#[cfg(all(feature = "json", feature = "coroutine"))]
pub async fn co_write_json<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    let path = path.as_ref();
    let content = crate::check!(
        crate::json::stringify(content),
        "failed to write json to file '{}'",
        path.display()
    )?;
    co_write(path, content).await
}

/// Serialize data as JSON, prettified, and write it to a file.
#[cfg(feature = "json")]
pub fn write_json_pretty<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    let path = path.as_ref();
    let content = crate::check!(
        crate::json::stringify_pretty(content),
        "failed to write json to file '{}'",
        path.display()
    )?;
    write(path, content)
}

/// Async version of [`write_json_pretty`]. The serialization part is synchronous, since it does not
/// involve IO.
#[cfg(all(feature = "json", feature = "coroutine"))]
pub async fn co_write_json_pretty<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    let path = path.as_ref();
    let content = crate::check!(
        crate::json::stringify_pretty(content),
        "failed to write json to file '{}'",
        path.display()
    )?;
    co_write(path, content).await
}

/// Serialize data as TOML and write it to a file.
#[cfg(feature = "toml")]
pub fn write_toml<S: serde::Serialize>(path: impl AsRef<Path>, content: &S) -> crate::Result<()> {
    let path = path.as_ref();
    let content = crate::check!(
        crate::toml::stringify(content),
        "failed to write toml to file '{}'",
        path.display()
    )?;
    write(path, content)
}

/// Async version of [`write_toml`]. The serialization part is synchronous, since it does not
/// involve IO.
#[cfg(all(feature = "toml", feature = "coroutine"))]
pub async fn co_write_toml<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    let path = path.as_ref();
    let content = crate::check!(
        crate::toml::stringify(content),
        "failed to write toml to file '{}'",
        path.display()
    )?;
    co_write(path, content).await
}

/// Serialize data as TOML, prettified, and write it to a file.
#[cfg(feature = "toml")]
pub fn write_toml_pretty<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    let path = path.as_ref();
    let content = crate::check!(
        crate::toml::stringify_pretty(content),
        "failed to write toml to file '{}'",
        path.display()
    )?;
    write(path, content)
}

/// Async version of [`write_toml_pretty`]. The serialization part is synchronous, since it does not
/// involve IO.
#[cfg(all(feature = "toml", feature = "coroutine"))]
pub async fn co_write_toml_pretty<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    let path = path.as_ref();
    let content = crate::check!(
        crate::toml::stringify_pretty(content),
        "failed to write toml to file '{}'",
        path.display()
    )?;
    co_write(path, content).await
}

/// Serialize data as YAML and write it to a file.
#[cfg(feature = "yaml")]
pub fn write_yaml<S: serde::Serialize>(path: impl AsRef<Path>, content: &S) -> crate::Result<()> {
    let path = path.as_ref();
    let content = crate::check!(
        crate::yaml::stringify(content),
        "failed to write yaml to file '{}'",
        path.display()
    )?;
    write(path, content)
}

/// Async version of [`write_yaml`]. The serialization part is synchronous, since it does not
/// involve IO.
#[cfg(all(feature = "yaml", feature = "coroutine"))]
pub async fn co_write_yaml<S: serde::Serialize>(
    path: impl AsRef<Path>,
    content: &S,
) -> crate::Result<()> {
    let path = path.as_ref();
    let content = crate::check!(
        crate::yaml::stringify(content),
        "failed to write yaml to file '{}'",
        path.display()
    )?;
    co_write(path, content).await
}
