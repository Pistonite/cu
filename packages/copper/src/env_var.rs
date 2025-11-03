use std::ffi::OsStr;

use crate::pre::*;

/// Like [`std::env::var`], but treat not-set as empty string.
/// Tracing and reporting is built-in.
///
/// If you need to detect the case where the env var is not set,
/// use `std::env::var` directly.
///
/// ```rust
/// # use pistonite_cu as cu;
/// # fn main() -> cu::Result<()> {
/// assert!(!cu::env_var("CARGO")?.is_empty());
/// assert!(cu::env_var("NOT_SET")?.is_empty());
/// # Ok(()) }
/// ```
#[inline(always)]
pub fn env_var(var: impl AsRef<OsStr>) -> crate::Result<String> {
    env_var_impl(var.as_ref())
}
fn env_var_impl(var: &OsStr) -> crate::Result<String> {
    crate::trace!("reading env var '{}`", var.display());
    match std::env::var(var) {
        Ok(v) => Ok(v),
        Err(std::env::VarError::NotPresent) => Ok(String::new()),
        Err(e) => {
            crate::rethrow!(e, "failed to read env var '{}'", var.display());
        }
    }
}
