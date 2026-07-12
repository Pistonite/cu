use std::ffi::OsStr;
use std::fs::{FileType, Metadata};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use ignore::{WalkBuilder as IgnoreWalkBuilder, Walk as IgnoreWalk, DirEntry as IgnoreDirEntry};
use ignore::overrides::OverrideBuilder;

use crate::pre::*;

/// Create a directory walker builder
#[inline(always)]
pub fn walker(root: impl AsRef<Path>) -> WalkBuilder {
    WalkBuilder::new(root.as_ref().to_path_buf())
}

/// Walk the directory with default settings:
/// - Include hidden files
/// - Does not use git-ignore or other ignore files
/// - Does not follow links
/// - Does not use glob to include or exclude files
#[inline(always)]
pub fn walk(root: impl AsRef<Path>) -> cu::Result<Walk> {
    walker(root).walk()
}

/// A wrapper for the walk builder that provides a simpler API for most
/// cases. See [`walker`]
pub struct WalkBuilder {
    inner: IgnoreWalkBuilder,
    overrides: OverrideBuilder,
    has_overrides: bool,
    include_dir_entries: bool,
    root: PathBuf,
}

impl WalkBuilder {
    fn new(root: PathBuf) -> Self {
        let mut inner = IgnoreWalkBuilder::new(root.clone());
        inner.require_git(true);
        inner.ignore(false);
        inner.hidden(false);
        let mut s = Self {
            inner,
            overrides: OverrideBuilder::new(root.clone()),
            has_overrides: false,
            include_dir_entries: false,
            root,
        };
        s.git(false);
        s
    }

    
    /// Return the inner WalkBuilder from the `ignore` crate for advanced configuration.
    /// Note that the `overrides` matcher will be replaced when building the walker
    pub fn as_inner_mut(&mut self) -> &mut IgnoreWalkBuilder {
        &mut self.inner
    }

    /// Add glob patterns to be included. By default all paths are included
    pub fn glob_includes(&mut self, globs: impl Iterator<Item=impl AsRef<str>>) -> crate::Result<&mut Self> {
        for g in globs {
            let g = g.as_ref();
            crate::check!(self.overrides.add(g), "failed to add glob include pattern: '{g}'")?;
            self.has_overrides = true;
        }
        Ok(self)
    }

    /// Add glob patterns to be excluded. By default none
    pub fn glob_excludes(&mut self, globs: impl Iterator<Item=impl AsRef<str>>) -> crate::Result<&mut Self> {
        let mut s = String::new();
        s.push('!');
        for g in globs {
            let g = g.as_ref();
            s.push_str(g);
            crate::check!(self.overrides.add(&s), "failed to add glob exclude pattern: '{s}'")?;
            s.truncate(1);
            self.has_overrides = true;
        }
        Ok(self)
    }

    /// Set if directory entries should be returned while iterating, default is false.
    /// When enabled, also reads the symlinks and ignore links to directories (but the files in
    /// the link target will still not be returned unless `follow_links` is enabled)
    #[inline(always)]
    pub fn include_dir_entries(&mut self, include: bool) -> &mut Self {
        self.include_dir_entries = include;
        self
    }

    /// Enable reading `.gitignore` and git exclude configs (mostly) the same behavior as git.
    /// Default is disabled.
    #[inline(always)]
    pub fn git(&mut self, yes: bool) -> &mut Self {
        self.inner.git_global(yes);
        self.inner.git_ignore(yes);
        self.inner.git_exclude(yes);
        self
    }

    /// Ignore hidden files. Default false
    #[inline(always)]
    pub fn ignore_hidden(&mut self, yes: bool) -> &mut Self {
        self.inner.hidden(yes);
        self
    }

    /// Follow symlinks. Default false
    pub fn follow_links(&mut self, yes: bool) -> &mut Self {
        self.inner.follow_links(yes);
        self
    }

    /// Add custom ignore file name
    #[inline(always)]
    pub fn add_ignore_filename(&mut self, ignore_file: &str) -> &mut Self {
        self.inner.add_custom_ignore_filename(ignore_file);
        self
    }

    /// Build the directory walker
    pub fn walk(mut self) -> cu::Result<Walk> {
        if self.has_overrides {
            let overrides = cu::check!(self.overrides.build(), "walk: failed to build glob pattern overrides")?;
            self.inner.overrides(overrides);
        }
        let walk = self.inner.build();
        Ok(Walk {
            inner: walk,
            include_dir_entries: self.include_dir_entries,
            root: Arc::new(self.root),
        })
    }
}

pub struct Walk {
    inner: IgnoreWalk,
    include_dir_entries: bool,
    root: Arc<PathBuf>
}

impl Iterator for Walk {
    type Item = crate::Result<WalkEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_internal() {
            Err(e) => Some(Err(e)),
            Ok(None) => None,
            Ok(Some(e)) => Some(Ok(e)),
        }
    }
}

impl Walk {
    fn next_internal(&mut self) -> crate::Result<Option<WalkEntry>> {
        let (entry, file_type) = loop {
            let entry = crate::some!(self.inner.next()); 
            let entry = crate::check!(entry, "walk: failed to read the next entry")?;
            match entry.file_type() {
                None => {
                    // stdin
                    continue;
                }
                Some(t) => {
                    if self.include_dir_entries {
                        break (entry, t);
                    }
                    // filter out dir entries
                    if t.is_file() {
                        break (entry, t);
                    }
                    if t.is_dir() {
                        crate::trace!("walk: skipping directory: '{}'", entry.path().display());
                        continue;
                    }
                    if t.is_symlink() && entry.path().is_dir() {
                        crate::trace!("walk: skipping symlinked directory: '{}'", entry.path().display());
                        continue;
                    }
                    crate::trace!("walk: skipping entry with unknown file type: '{}'", entry.path().display());
                    continue;
                }
            }
        };
        Ok(Some(WalkEntry {
            root: Arc::clone(&self.root),
            inner: entry,
            file_type
        }))
    }
}

pub struct WalkEntry {
    root: Arc<PathBuf>,
    inner: IgnoreDirEntry,
    file_type: FileType,
}

impl WalkEntry {
    /// Get the root of the walk
    #[inline(always)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the depth of this entry, `0` is root, `1` is an entry in the root, etc.
    pub fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Get the path by joining the walk root and the relative
    /// path of the entry
    #[inline(always)]
    pub fn path(&self) -> &Path {
        self.inner.path()
    }

    /// Get the relative path of this entry from the walk root, without leading `./`
    ///
    /// Return `None` if the entry is root
    pub fn rel_path(&self) -> cu::Result<Option<PathBuf>> {
        // ensure root is a prefix of inner path
        let root_norm = self.root.normalize()?;
        // note we cannot normalize the path after join since it might be a symlink
        let path_norm = root_norm.join(self.inner.path());
        let mut root_iter = root_norm.components().filter(|x| !matches!(x, Component::CurDir));
        let mut path_iter = path_norm.components().filter(|x| !matches!(x, Component::CurDir));
        loop {
            let Some(root_comp) = root_iter.next() else {
                break;
            };
            let path_comp = cu::check!(path_iter.next(), 
                "unexpected: walk entry path is shorter than root")?;
            cu::ensure!(root_comp == path_comp,
                "unexpected: walk entry path is not in root")?;
        }
        let Some(next) = path_iter.next() else {
            // is root
            return Ok(None);
        };
        let mut p = PathBuf::new();
        p.push(next);
        p.extend(path_iter);
        Ok(Some(p))
    }

    /// Get the file type
    #[inline(always)]
    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    /// Check if the entry is a file.
    #[inline(always)]
    pub fn is_file(&self) -> bool {
        self.file_type.is_file()
    }

    /// Check if the entry is a directory.
    #[inline(always)]
    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }

    /// Check if the entry is a symlink.
    #[inline(always)]
    pub fn is_symlink(&self) -> bool {
        self.file_type.is_symlink()
    }

    /// Get the file name
    #[inline(always)]
    pub fn file_name(&self) -> Option<&OsStr> {
        self.inner.path().file_name()
    }

    /// Get the entry metadata
    pub fn metadata(&self) -> crate::Result<Metadata> {
        crate::check!(
            self.inner.metadata(),
            "failed to get metadata for file '{}' while walking directory '{}'",
            self.inner.path().try_to_rel_from(&*self.root).display(),
            self.root.display()
        )
    }

}
