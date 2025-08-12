use std::path::PathBuf;

use crate::pre::*;

/// `std::env::current_exe()` with error reporting
///
/// ```rust
/// # fn main() -> cu::Result<()> {
/// let path = cu::fs::current_exe()?;
/// assert!(path.is_absolute());
/// # Ok(()) }
/// ```
pub fn current_exe() -> crate::Result<PathBuf> {
    std::env::current_exe().context("failed to get current exe path")
}

// WIP
// Move a file from
// pub fn move_file(from: impl AsRef<Path>, to: impl AsRef<Path>)
