//! Binary path registry
//!
//! This util to provide a unified way of getting the path of a program to run,
//! for example, let's say you are making a build script for a project, and
//! you want to find the compiler program `gcc` in the following manner:
//! - Most users would use the project-provided toolchain at `./toolchain/compiler/gcc`
//! - If the user want to use the compiler on their system instead, we would first try
//!   to finding `gcc` in `PATH`, then fallback to the toolchain.
//! - Or some user may want to specify a particular path on their system with the `CC` environment
//!   variable. We would try this first
//!
//! With `cu::bin`, you can register this logic once, then just use [`cu::which`](function@which)
//! whenever you need to get the resolved path:
//! ```rust,no_run
//! use std::path::PathBuf;
//! use cu::pre::*;
//!
//! fn find_gcc(use_system: bool) -> cu::Result<PathBuf> {
//!     let provided = cu::bin::location("./toolchain/compiler/gcc".as_ref());
//!     if use_system {
//!         cu::bin::find("gcc", [
//!             cu::bin::from_env("CC"),
//!             cu::bin::in_PATH(),
//!             provided
//!         ])
//!     } else {
//!         cu::bin::find("gcc", [
//!             cu::bin::from_env("CC"),
//!             provided
//!         ])
//!     }
//! }
//!
//! # fn main() -> cu::Result<()> {
//! // would use clap in a real program, here, just an example
//! let use_system = std::env::args().any(|x| x == "--use-system-compiler");
//! // register the path with the logic in find_gcc
//! let _: PathBuf = find_gcc(use_system).context("cannot find gcc")?;
//!
//! // ... later in the progam
//! cu::which("gcc")?.command()
//!     //... configure the command
//! #        ;
//! # Ok(())
//! # }
//!
//! ```
//!
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};

use crate::PathExtension as _;

static BIN_PATHS: LazyLock<RwLock<BTreeMap<String, PathBuf>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));

/// Check and set bin name to the specified path. Return the absolute path that is set.
///
/// This is different from the other functions in the module, as it will overwrite
/// the existing cache entry.
///
/// See [`cu::bin`](self) module-level documentation for more info.
///
/// # Example
/// ```rust,no_run
/// use std::path::PathBuf;
///
/// # fn main() -> cu::Result<()> {
/// assert_eq!(cu::which("ninja")?, PathBuf::from("/usr/bin/ninja"));
/// cu::bin::set("ninja", "/my/path/to/ninja")?;
/// // `which` now returns registered value
/// assert_eq!(cu::which("ninja")?, PathBuf::from("/my/path/to/ninja"));
/// # Ok(())
/// # }
/// ```
pub fn set(name: impl AsRef<str>, path: impl AsRef<Path>) -> crate::Result<PathBuf> {
    let name = name.as_ref();
    let path = location(path.as_ref()).find(name)?;
    crate::trace!("setting bin path: '{name}' -> '{}'", path.display());
    let mut paths = BIN_PATHS
        .write()
        .expect("could not lock global bin path map");
    let old = paths.insert(name.to_string(), path.clone());
    if let Some(old) = old {
        crate::trace!("replacing old bin path: '{}'", old.display());
    }
    Ok(path)
}

/// Find an executable in PATH, or use a previously registered path (also available as
/// `cu::which`).
///
/// Result will be cached so finding the same executable name will always result in the same path.
///
/// See [`cu::bin`](self) module-level documentation for more info.
pub fn which(name: impl AsRef<str>) -> crate::Result<PathBuf> {
    find(name, std::iter::once(in_PATH()))
}

/// Resolve an executable at the location provided, or use a previously registered path.
///
/// Result will be cached so finding the same executable name will always result in the same path.
///
/// See [`cu::bin`](self) module-level documentation for more info.
pub fn resolve(name: impl AsRef<str>, path: impl AsRef<Path>) -> crate::Result<PathBuf> {
    find(name, std::iter::once(location(path.as_ref())))
}

/// Find the executable using a series of strategies.
///
/// If there's already a registered path from a previous find that matches `name`,
/// that path is returned instead.
///
/// See [`cu::bin`](self) module-level documentation for more info.
pub fn find<'a, I: IntoIterator<Item = Strategy<'a>>>(
    name: impl AsRef<str>,
    strats: I,
) -> crate::Result<PathBuf> {
    let name = name.as_ref();
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
    let path = find_with_strats(name, strats)?;
    crate::trace!("found executable '{name}': {}", path.display());
    paths.insert(name.to_string(), path.clone());
    Ok(path)
}

fn find_with_strats<'a, I: IntoIterator<Item = Strategy<'a>>>(
    name: &str,
    strats: I,
) -> crate::Result<PathBuf> {
    let mut strats = strats.into_iter();
    let Some(first) = strats.next() else {
        crate::bail!("must provide at least one strategy to cu::bin::find");
    };
    let mut error = match first.find(name) {
        Ok(x) => return Ok(x),
        Err(e) => e,
    };
    let mut prev = first;
    for strat in strats {
        crate::trace!("could not finding '{name}' {prev}: {error}");
        crate::trace!("falling back to finding '{name}' {strat}");
        error = match strat.find(name) {
            Ok(x) => return Ok(x),
            Err(e) => e,
        };
        prev = strat;
    }

    crate::trace!("could not finding '{name}' {prev}: {error}");
    crate::bailand!(error!("could not find program '{name}'"));
}

/// Strategy to resolve the given path as path for the program
///
/// See [`cu::bin`](self) module-level documentation for more info.
pub fn location<'a>(path: &'a Path) -> Strategy<'a> {
    Strategy::Resolve(path)
}

/// Strategy to find the program in the PATH environment variable
///
/// See [`cu::bin`](self) module-level documentation for more info.
#[allow(non_snake_case)]
pub fn in_PATH() -> Strategy<'static> {
    Strategy::Which
}
/// Strategy to read the environment variable and resolve that path as the program
///
/// See [`cu::bin`](self) module-level documentation for more info.
pub fn from_env<'a>(env: &'a str) -> Strategy<'a> {
    Strategy::EnvVar(env)
}

/// Strategy to find a binary
///
/// See [`cu::bin`](self) module-level documentation for more info.
pub enum Strategy<'a> {
    /// Find it in PATH (like the `which` command)
    Which,
    /// Resolve a provided path
    Resolve(&'a Path),
    /// Resolve from path in an environment variable
    EnvVar(&'a str),
}

impl Strategy<'_> {
    fn find(&self, name: &str) -> crate::Result<PathBuf> {
        match self {
            Self::Which => {
                crate::trace!("finding executable '{name}' in PATH");
                match which::which(name) {
                    // which already canonicalize it, which ensures it exists
                    Ok(x) => Ok(x.simplified().to_path_buf()),
                    Err(e) => Err(e)?,
                }
            }
            Self::Resolve(path) => {
                crate::trace!("finding executable '{name}' at '{}'", path.display());
                path.normalize_executable()
            }
            Self::EnvVar(v) => {
                crate::trace!("finding executable '{name}' using environment variable '{v}'");
                match std::env::var(v) {
                    Ok(x) if !x.is_empty() => Path::new(&x).normalize_executable(),
                    _ => crate::bail!("environment variable '{v}' not found"),
                }
            }
        }
    }
}
impl std::fmt::Display for Strategy<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Strategy::Which => write!(f, "in PATH"),
            Strategy::Resolve(path) => write!(f, " at '{}'", path.display()),
            Strategy::EnvVar(env) => write!(f, "using envvar '{env}'"),
        }
    }
}
