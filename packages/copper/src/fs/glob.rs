use std::path::{Path, PathBuf};

use crate::pre::*;

/// Resolve glob pattern from a base path.
///
/// The pattern is joined onto the base path. If the pattern
/// is absolute, then the base path has no effect, as it will
/// be replaced by the pattern.
///
/// This is a thin wrapper around rust-lang's `glob` crate.
/// This returns an iterator over the paths that match
/// the glob pattern.
///
/// If the resulting pattern is absolute, then it will only match absolute
/// paths that matches the pattern (rather than the behavior
/// of, for example, `.gitignore`)
pub fn glob_from(path: impl AsRef<Path>, pattern: &str) -> crate::Result<Glob> {
    let path = path.as_ref().join(pattern);
    let pattern = crate::check!(
        path.into_utf8(),
        "base path is not UTF-8 while trying to glob: {pattern}"
    )?;
    glob(&pattern)
}

/// Resolve glob pattern from the current directory.
///
/// This is a thin wrapper around rust-lang's `glob` crate.
/// This returns an iterator over the paths that match
/// the glob pattern.
///
/// If the pattern is absolute, then it will only match absolute
/// paths that matches the pattern (rather than the behavior
/// of, for example, `.gitignore`)
pub fn glob(pattern: &str) -> crate::Result<Glob> {
    let iter = crate::check!(
        ::glob::glob(pattern),
        "failed to parse glob pattern: {pattern}"
    )?;
    Ok(Glob(iter))
}

/// Iterator for [`glob`] and [`glob_from`]
///
/// This is a thin wrapper for rust-lang's `glob` crate
/// that converts the error to anyhow.
pub struct Glob(::glob::Paths);
impl Iterator for Glob {
    type Item = crate::Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next()? {
            Ok(p) => Some(Ok(p)),
            Err(e) => {
                let path = e.path();
                let outer = crate::fmterr!("glob: cannot read '{}'", path.display());
                Some(Err(e.into_error()).context(outer))
            }
        }
    }
}
