use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};

use crate::{Context as _, PathExtension as _};

static BIN_PATHS: LazyLock<RwLock<BTreeMap<String, PathBuf>>> = LazyLock::new(|| RwLock::new(BTreeMap::new()));

/// Check and set bin name to the specified path. Return the absolute path that is set.
///
/// This is different from the other `bin::` functions, as it will overwrite
/// the existing cache entry.
///
/// # Example
/// ```rust,no_run
/// cu::bin::set("ninja", "/my/path/to/ninja").unwrap();
/// assert_eq!(cu::bin::which("ninja").unwrap(), PathBuf::new("/my/path/to/ninja"));
/// ```
pub fn set(name: impl AsRef<str>, path: impl AsRef<Path>) -> crate::Result<PathBuf> {
    let name = name.as_ref();
    let path = direct_strategy(name, path.as_ref())?;
    crate::debug!("setting bin path: '{name}' -> '{}'", path.display());
    let mut paths = BIN_PATHS.write().expect("could not lock global bin path map");
    let old = paths.insert(name.to_string(), path.clone());
    if let Some(old) = old {
        crate::debug!("replacing old bin path: '{}'", old.display());
    }
    Ok(path)
}

/// Find an executable in PATH or previously cached.
///
/// Result will be cached so finding the same executable name will always result in the same path
pub fn which(name: impl AsRef<str>) -> crate::Result<PathBuf> {
    let name = name.as_ref();
    find_bin_internal(name, which_strategy).with_context(|| format!("could not find executable '{name}'"))
}

/// Find an executable in PATH or previously cached, falling back
/// to reading from environment variable `env` if not found.
///
/// Result will be cached so finding the same executable name will always result in the same path
pub fn which_or_env(name: impl AsRef<str>, env: impl AsRef<str>) -> crate::Result<PathBuf> {
    let name = name.as_ref();
    let env = env.as_ref();
    find_bin_internal(name,
        |name| match which_strategy(name) {
            Ok(x) => Ok(x),
            Err(e) => {
                crate::debug!("could not find executable '{name}' in PATH: {e}");
                crate::debug!("falling back to finding '{name}' in environment");
                env_strategy(name, env)
            }
        }
    ).with_context(|| format!("could not find executable '{name}'"))
}

/// Use the path defined in environment variable `env` as the path for the binary,
/// or fallback to finding it in PATH.
///
/// Result will be cached so finding the same executable name will always result in the same path
pub fn env_or_which(name: impl AsRef<str>, env: impl AsRef<str>) -> crate::Result<PathBuf> {
    let name = name.as_ref();
    let env = env.as_ref();
    find_bin_internal(name,
        |name| match env_strategy(name, env) {
            Ok(x) => Ok(x),
            Err(e) => {
                crate::debug!("could not find executable '{name}' in environment: {e}");
                crate::debug!("falling back to finding '{name}' in PATH");
                which_strategy(name)
            }
        }
    ).with_context(|| format!("could not find executable '{name}'"))
}

/// Register `path` as the path for the binary `name` if it exists,
/// otherwise falling back to finding it in PATH.
///
/// Result will be cached so finding the same executable name will always result in the same path
pub fn resolve_or_which(name: impl AsRef<str>, path: impl AsRef<Path>) -> crate::Result<PathBuf> {
    let name = name.as_ref();
    let path = path.as_ref();
    find_bin_internal(name,
        |name| match direct_strategy(name, path) {
            Ok(x) => Ok(x),
            Err(e) => {
                crate::debug!("failed to resolve executable '{name}' at '{}': {e}", path.display());
                crate::debug!("falling back to finding '{name}' in PATH");
                which_strategy(name)
            }
        }
    ).with_context(|| format!("could not find executable '{name}'"))
}

/// Resolve binary path for `name` in the following order:
/// - Use `path` canonicalized
/// - Use environment variable `env`
/// - find in PATH
///
/// Result will be cached so finding the same executable name will always result in the same path
pub fn resolve_or_env_or_which(name: impl AsRef<str>, path: impl AsRef<Path>, env: impl AsRef<str>) -> crate::Result<PathBuf> {
    let name = name.as_ref();
    let path = path.as_ref();
    let env = env.as_ref();
    find_bin_internal(name,
        |name| {
            match direct_strategy(name, path) {
                Ok(x) => return Ok(x),
                Err(e) => {
                    crate::debug!("failed to resolve executable '{name}' at '{}': {e}", path.display());
                    crate::debug!("falling back to finding '{name}' in environment");
                }
            }
            match env_strategy(name, env) {
                Ok(x) => return Ok(x),
                Err(e) => {
                    crate::debug!("failed to resolve executable '{name}' in environment: {e}");
                    crate::debug!("falling back to finding '{name}' in PATH");
                }
            }
            which_strategy(name)
        }
    ).with_context(|| format!("could not find executable '{name}'"))
}

/// Resolve binary path for `name` in the following order:
/// - Use `path` canonicalized
/// - Use environment variable `env`
/// - find in PATH
///
/// Result will be cached so finding the same executable name will always result in the same path
pub fn resolve_or_which_or_env(name: impl AsRef<str>, path: impl AsRef<Path>, env: impl AsRef<str>) -> crate::Result<PathBuf> {
    let name = name.as_ref();
    let path = path.as_ref();
    let env = env.as_ref();
    find_bin_internal(name,
        |name| {
            match direct_strategy(name, path) {
                Ok(x) => return Ok(x),
                Err(e) => {
                    crate::debug!("failed to resolve executable '{name}' at '{}': {e}", path.display());
                    crate::debug!("falling back to finding '{name}' in PATH");
                }
            }
            match which_strategy(name) {
                Ok(x) => return Ok(x),
                Err(e) => {
                    crate::debug!("failed to resolve executable '{name}' in PATH: {e}");
                    crate::debug!("falling back to finding '{name}' in environment");
                }
            }
            env_strategy(name, env)
        }
    ).with_context(|| format!("could not find executable '{name}'"))
}

/// Find the absolute path given the binary name. Return error if cannot find.
fn find_bin_internal(name: &str, strategy: impl FnOnce(&str) -> crate::Result<PathBuf>) -> crate::Result<PathBuf> {
    {
        let Ok(paths) = BIN_PATHS.read() else {
            crate::bail!("could not lock global bin path map");
        };
        if let Some(x) = paths.get(name) {
            return Ok(x.clone());
        }
    }
    let Ok(mut paths) = BIN_PATHS.write() else {
        crate::bail!("could not lock global bin path map");
    };
    if let Some(x) = paths.get(name) {
        return Ok(x.clone());
    }
    let path = strategy(name)?;
    crate::debug!("found executable '{name}' -> '{}'", path.display());
    paths.insert(name.to_string(), path.clone());
    Ok(path)
}

fn env_strategy(name: &str, env: &str) -> crate::Result<PathBuf> {
    crate::trace!("finding executable '{name}' using environment variable '{env}'");
    match std::env::var(env) {
        Ok(x) if !x.is_empty() => Path::new(&x).normalize_exists(),
        _ => crate::bail!("environment variable '{env}' not found"),
    }
}

fn which_strategy(name: &str) -> crate::Result<PathBuf> {
    crate::trace!("finding executable '{name}' in PATH");
    match which::which(name) {
        // which already canonicalize it, which ensures it exists
        Ok(x) => x.normalize(),
        Err(e) => Err(e)?
    }
}

fn direct_strategy(name: &str, path: &Path) -> crate::Result<PathBuf> {
    crate::trace!("finding executable '{name}' at '{}'", path.display());
    path.normalize_exists()
}
