use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crate::Context as _;

/// Extension to paths
///
/// Most of these are related to file system, and not purely path processing.
/// Therefore this is tied to the `fs` feature.
pub trait PathExtension {
    /// Get file name. Error if the file name is not UTF-8 or other error occurs
    fn file_name_str(&self) -> crate::Result<&str>;

    /// Check that the path exists, or fail with an error
    fn check_exists(&self) -> crate::Result<()>;

    /// Return the simplified path if the path has a Windows UNC prefix
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
    fn normalize(&self) -> crate::Result<PathBuf>;

    /// Like `normalize`, but with the additional guarantee that the path exists
    fn normalize_exists(&self) -> crate::Result<PathBuf> {
        let x = self.normalize()?;
        x.check_exists()?;
        Ok(x)
    }

    /// Like `normalize`, but with the additional guarantee that:
    /// - The file name of the output will be the same as the input
    /// - The path exists and is not a directory
    fn normalize_executable(&self) -> crate::Result<PathBuf>;

    /// Get the parent path as an absolute path
    ///
    /// Path navigation is very complex and that's why we are paying a little performance
    /// cost and always returning `PathBuf`, and always converting the path to absolute.
    ///
    /// For path manipulating (i.e. as a OsStr), instead of navigation, use std `parent()`
    /// instead
    fn parent_abs(&self) -> crate::Result<PathBuf> {
        self.parent_abs_times(1)
    }

    /// Effecitvely chaining `parent_abs` `x` times
    fn parent_abs_times(&self, x: usize) -> crate::Result<PathBuf>;

    /// Try converting self to a relative path from current working directory.
    ///
    /// Return the path itself unmodified if conversion failed
    fn try_to_rel(&self) -> Cow<'_, Path> {
        self.try_to_rel_from(".")
    }

    /// Try converting self to a relative path from a base path
    fn try_to_rel_from(&self, path: impl AsRef<Path>) -> Cow<'_, Path>;

    /// Start building a child process with the path as the executable
    #[cfg(feature = "process")]
    fn command(&self) -> crate::CommandBuilder;
}

impl PathExtension for Path {
    fn file_name_str(&self) -> crate::Result<&str> {
        let file_name = self
            .file_name()
            .with_context(|| format!("cannot get file name for path: {}", self.display()))?;
        // to_str is ok on all platforms, because Rust internally
        // represent OsStrings on Windows as WTF-8
        // see https://doc.rust-lang.org/src/std/sys_common/wtf8.rs.html
        let Some(file_name) = file_name.to_str() else {
            crate::bail!("file name is not valid UTF-8: {}", self.display());
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

    fn check_exists(&self) -> crate::Result<()> {
        if !self.exists() {
            crate::bail!("path '{}' does not exist.", self.display());
        }
        Ok(())
    }

    fn normalize(&self) -> crate::Result<PathBuf> {
        if let Ok(x) = dunce::canonicalize(self) {
            return Ok(x);
        };
        if self.is_absolute() {
            return fallback_normalize_absolute(self);
        }

        let Ok(mut base) = dunce::canonicalize(".") else {
            crate::warn!("failed to normalize current directory");
            crate::bail!(
                "cannot normalize current directory when normalizing relative path: {}",
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
                crate::warn!("failed to normalize current directory");
                crate::bail!(
                    "cannot normalize current directory when normalizing relative path: {}",
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

    fn try_to_rel_from(&self, path: impl AsRef<Path>) -> Cow<'_, Path> {
        let path = path.as_ref();
        let res = match (self.is_absolute(), path.is_absolute()) {
            (true, true) => pathdiff::diff_paths(self, path),
            (true, false) => {
                let Ok(base) = path.normalize() else {
                    return Cow::Borrowed(self);
                };
                pathdiff::diff_paths(self, base.as_path())
            }
            (false, true) => {
                let Ok(self_) = self.normalize() else {
                    return Cow::Borrowed(self);
                };
                pathdiff::diff_paths(self_.as_path(), path)
            }
            (false, false) => {
                let Ok(self_) = self.normalize() else {
                    return Cow::Borrowed(self);
                };
                let Ok(base) = path.normalize() else {
                    return Cow::Borrowed(self);
                };
                pathdiff::diff_paths(self_.as_path(), base.as_path())
            }
        };
        match res {
            None => Cow::Borrowed(self),
            Some(x) => Cow::Owned(x),
        }
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
        Some(prefix) => PathBuf::from(prefix.as_os_str().to_ascii_uppercase()),
    };
    out.extend(components);
    // simplify windows UNC if needed
    if out.as_os_str().as_encoded_bytes().starts_with(b"\\\\") {
        Ok(dunce::simplified(&out).to_path_buf())
    } else {
        Ok(out)
    }
}

macro_rules! impl_for_as_ref_path {
    ($type:ty) => {
        impl PathExtension for $type {
            fn file_name_str(&self) -> crate::Result<&str> {
                AsRef::<Path>::as_ref(self).file_name_str()
            }
            fn simplified(&self) -> &Path {
                AsRef::<Path>::as_ref(self).simplified()
            }
            fn normalize(&self) -> crate::Result<PathBuf> {
                AsRef::<Path>::as_ref(self).normalize()
            }
            fn normalize_executable(&self) -> crate::Result<PathBuf> {
                AsRef::<Path>::as_ref(self).normalize_executable()
            }
            fn check_exists(&self) -> crate::Result<()> {
                AsRef::<Path>::as_ref(self).check_exists()
            }
            fn parent_abs_times(&self, x: usize) -> crate::Result<PathBuf> {
                AsRef::<Path>::as_ref(self).parent_abs_times(x)
            }
            fn try_to_rel_from(&self, path: impl AsRef<Path>) -> Cow<'_, Path> {
                AsRef::<Path>::as_ref(self).try_to_rel_from(path)
            }
            #[cfg(feature = "process")]
            fn command(&self) -> crate::CommandBuilder {
                AsRef::<Path>::as_ref(self).command()
            }
        }
    };
}

impl_for_as_ref_path!(PathBuf);
