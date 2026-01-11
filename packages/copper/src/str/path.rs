use std::borrow::Cow;
use std::path::{Path, PathBuf};

use cu::Context as _;

/// # File System Paths and Strings
/// Rust works with [`String`](std::string)s, which are UTF-8 encoded bytes.
/// However, not all operating systems work with UTF-8. That's why Rust has
/// [`OsString`](std::ffi::OsString), which has platform-specific implementations.
/// And `PathBuf`s are wrappers for `OsString`.
///
/// However, often when writing platform-independent code, we want to stay
/// in the UTF-8 realm, but conversion can be painful because you must handle
/// the error when the `OsString` is not valid UTF-8.
///
/// ```rust
/// # use pistonite_cu as cu;
/// use std::ffi::{OsString, OsStr};
///
/// use cu::pre::*;
///
/// fn take_os_string(s: &OsStr) -> cu::Result<()> {
///     match s.to_str() {
///         Some(s) => {
///             cu::info!("valid utf-8: {s}");
///             Ok(())
///         }
///         None => {
///             cu::bail!("not valid utf-8!");
///         }
///     }
/// }
/// ```
///
/// `cu` provides extension traits that integrates with `cu::Result`,
/// so you can have the error handling by simply propagate with `?`.
///
/// There are 4 traits, all will be included into scope with `use cu::pre::*;`.
/// The path extensions also have utilities for working with file system specifically,
/// (such as normalizing it), which is why they require the `fs` feature.
///
/// - [`OsStrExtension`](trait@crate::str::OsStrExtension)
/// - [`OsStrExtensionOwned`](trait@crate::str::OsStrExtensionOwned)
/// - `PathExtension` - requires `fs` feature
pub trait PathExtension {
    /// Get file name. Error if the file name is not UTF-8 or other error occurs
    fn file_name_str(&self) -> cu::Result<&str>;

    /// Check that the path exists, or fail with an error
    fn ensure_exists(&self) -> cu::Result<()>;

    /// Return the simplified path if the path has a Windows UNC prefix
    ///
    /// The behavior is the same cross-platform.
    fn simplified(&self) -> &Path;

    /// Get absolute path for a path.
    ///
    /// This is different from `canonicalize()` since that fails for paths that don't
    /// exist. Here, we do a fallback that normalizes the path manually if `canonicalize()`
    /// fails:
    /// - If the path is absolute, then empty, `.`, and `..` path segments are normalized and removed,
    ///   and error if it tries to get the parent of root at some point.
    /// - If the path is relative, then the path is normalized the same way as above,
    ///   and then appended to the absolute path of the current working directory.
    ///
    /// On Windows only, it returns the most compatible form using the `dunce` crate instead of
    /// UNC, and the drive letter is also normalized to upper case
    fn normalize(&self) -> cu::Result<PathBuf>;

    /// Like `normalize`, but with the additional guarantee that the path exists
    fn normalize_exists(&self) -> cu::Result<PathBuf> {
        let x = self.normalize()?;
        x.ensure_exists()?;
        Ok(x)
    }

    /// Like `normalize`, but with the additional guarantee that:
    /// - The file name of the output will be the same as the input. This is because
    ///   the executable can be a multicall binary that behaves differently
    ///   depending on the executable name.
    /// - The path exists and is not a directory
    fn normalize_executable(&self) -> cu::Result<PathBuf>;

    /// Get the parent path as an absolute path
    ///
    /// Path navigation is very complex and that's why we are paying a little performance
    /// cost and always returning `PathBuf`, and always converting the path to absolute.
    ///
    /// For path manipulation (i.e. as a OsStr), instead of navigation, use std `parent()`
    /// instead
    #[inline(always)]
    fn parent_abs(&self) -> cu::Result<PathBuf> {
        self.parent_abs_times(1)
    }

    /// Effecitvely chaining `parent_abs` `x` times
    fn parent_abs_times(&self, x: usize) -> cu::Result<PathBuf>;

    /// Try converting self to a relative path from current working directory.
    ///
    /// Return the path itself unmodified if conversion failed
    fn try_to_rel(&self) -> Cow<'_, Path> {
        self.try_to_rel_from(".")
    }

    /// Try converting self to a relative path from a base path
    fn try_to_rel_from(&self, path: impl AsRef<Path>) -> Cow<'_, Path>;

    /// Start building a child process with the path as the executable
    ///
    /// See [Spawn Commands](crate::CommandBuilder)
    #[cfg(feature = "process")]
    fn command(&self) -> cu::CommandBuilder;
}

impl PathExtension for Path {
    fn file_name_str(&self) -> cu::Result<&str> {
        let file_name = self
            .file_name()
            .with_context(|| format!("cannot get file name for path: '{}'", self.display()))?;
        // to_str is ok on all platforms, because Rust internally
        // represent OsStrings on Windows as WTF-8
        // see https://doc.rust-lang.org/src/std/sys_common/wtf8.rs.html
        let Some(file_name) = file_name.to_str() else {
            crate::bail!("file name is not utf-8: '{}'", self.display());
        };
        Ok(file_name)
    }

    fn simplified(&self) -> &Path {
        if self.as_os_str().as_encoded_bytes().starts_with(b"\\\\") {
            dunce::simplified(self)
        } else {
            self
        }
    }

    fn ensure_exists(&self) -> cu::Result<()> {
        if !self.exists() {
            crate::bail!("path '{}' does not exist.", self.display());
        }
        Ok(())
    }

    fn normalize(&self) -> cu::Result<PathBuf> {
        if let Ok(x) = dunce::canonicalize(self) {
            return Ok(x);
        };
        if self.is_absolute() {
            return fallback_normalize_absolute(self);
        }

        let Ok(mut base) = dunce::canonicalize(".") else {
            crate::bail!(
                "failed to normalize current directory when normalizing relative path: '{}'",
                self.display()
            );
        };

        base.push(self);
        fallback_normalize_absolute(&base)
    }

    fn normalize_executable(&self) -> crate::Result<PathBuf> {
        // canonicalize will resolve symlinks, which will destroy
        // the file name, so we cannot use that
        let absolute_self = if self.is_absolute() {
            fallback_normalize_absolute(self)?
        } else {
            let Ok(mut base) = dunce::canonicalize(".") else {
                crate::bail!(
                    "failed to normalize current directory when normalizing relative path: '{}'",
                    self.display()
                );
            };

            base.push(self);
            fallback_normalize_absolute(&base)?
        };
        if !absolute_self.exists() {
            crate::bail!(
                "failed to normalize executable path '{}': does not exist",
                absolute_self.display()
            );
        }
        if absolute_self.is_dir() {
            crate::bail!(
                "failed to normalize executable path '{}': is a directory",
                absolute_self.display()
            )
        }
        Ok(absolute_self)
    }

    fn parent_abs_times(&self, x: usize) -> crate::Result<PathBuf> {
        let mut out = self.normalize()?;

        // this is correct since out is normalized
        for _ in 0..x {
            if !out.pop() {
                crate::bail!("trying to get parent of root");
            }
        }
        Ok(out)
    }

    #[inline(always)]
    fn try_to_rel_from(&self, path: impl AsRef<Path>) -> Cow<'_, Path> {
        try_to_rel_from(self, path.as_ref())
    }

    #[cfg(feature = "process")]
    fn command(&self) -> crate::CommandBuilder {
        crate::CommandBuilder::new(self)
    }
}

/// Normalize a path that failed to canonicalize
fn fallback_normalize_absolute(path: &Path) -> crate::Result<PathBuf> {
    let mut prefix = None;
    let mut components = vec![];
    for c in path.components() {
        match c {
            std::path::Component::Prefix(_prefix) => {
                prefix = Some(_prefix);
            }
            std::path::Component::RootDir => {
                components.clear();
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if components.pop().is_none() {
                    crate::bail!(
                        "trying to get parent of root when normalizing: {}",
                        path.display()
                    );
                }
            }
            std::path::Component::Normal(os_str) => components.push(os_str),
        }
    }
    let mut out = match prefix {
        None => PathBuf::from("/"),
        Some(prefix) => {
            let mut out = prefix.as_os_str().to_ascii_uppercase();
            out.push("\\"); // ok since this only occurs on windows
            out.into()
        }
    };
    out.extend(components);
    // simplify windows UNC if needed
    if out.as_os_str().as_encoded_bytes().starts_with(b"\\\\") {
        Ok(dunce::simplified(&out).to_path_buf())
    } else {
        Ok(out)
    }
}

fn try_to_rel_from<'a>(self_: &'a Path, path: &Path) -> Cow<'a, Path> {
    let res = match (self_.is_absolute(), path.is_absolute()) {
        (true, true) => pathdiff::diff_paths(self_, path),
        (true, false) => {
            let Ok(base) = path.normalize() else {
                return Cow::Borrowed(self_);
            };
            pathdiff::diff_paths(self_, base.as_path())
        }
        (false, true) => {
            let Ok(self_) = self_.normalize() else {
                return Cow::Borrowed(self_);
            };
            pathdiff::diff_paths(self_.as_path(), path)
        }
        (false, false) => {
            let Ok(self_abs) = self_.normalize() else {
                return Cow::Borrowed(self_);
            };
            let Ok(base) = path.normalize() else {
                return Cow::Borrowed(self_);
            };
            pathdiff::diff_paths(self_abs.as_path(), base.as_path())
        }
    };
    match res {
        None => Cow::Borrowed(self_),
        Some(x) => Cow::Owned(x),
    }
}

impl PathExtension for PathBuf {
    fn file_name_str(&self) -> crate::Result<&str> {
        self.as_path().file_name_str()
    }
    fn simplified(&self) -> &Path {
        self.as_path().simplified()
    }
    fn normalize(&self) -> crate::Result<PathBuf> {
        self.as_path().normalize()
    }
    fn normalize_executable(&self) -> crate::Result<PathBuf> {
        self.as_path().normalize_executable()
    }
    fn ensure_exists(&self) -> crate::Result<()> {
        self.as_path().ensure_exists()
    }
    fn parent_abs_times(&self, x: usize) -> crate::Result<PathBuf> {
        self.as_path().parent_abs_times(x)
    }
    fn try_to_rel_from(&self, path: impl AsRef<Path>) -> Cow<'_, Path> {
        self.as_path().try_to_rel_from(path)
    }
    #[cfg(feature = "process")]
    fn command(&self) -> crate::CommandBuilder {
        self.as_path().command()
    }
}
