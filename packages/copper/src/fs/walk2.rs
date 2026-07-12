//! Recursive directory walking.
//!
//! A thin, opinionated wrapper around the [`ignore`](https://docs.rs/ignore)
//! crate that exposes a simpler builder API for the most common cases.
//!
//! Use [`walk`] for a quick recursive walk with sensible defaults, or [`walker`]
//! to configure the walk through a [`WalkBuilder`] before running it.
//!
//! # Defaults
//! Out of the box (see [`walk`]) the walker:
//! - **includes** hidden files (dotfiles),
//! - does **not** read `.gitignore` or other ignore/VCS files,
//! - does **not** follow symbolic links,
//! - applies **no** glob include/exclude filters,
//! - yields **files only** — directory entries are skipped.
//!
//! Every one of these can be changed through [`WalkBuilder`].
//!
//! # Examples
//!
//! Walk the current directory and print every file:
//! ```rust,no_run
//! # use pistonite_cu as cu;
//! fn print_files() -> cu::Result<()> {
//!     for entry in cu::fs::walk(".")? {
//!         let entry = entry?;
//!         cu::info!("{}", entry.path().display());
//!     }
//!     Ok(())
//! }
//! ```
//!
//! Configure the walk with the builder — respect `.gitignore`, follow symlinks,
//! and only include Rust and TOML files (note brace alternation is supported):
//! ```rust,no_run
//! # use pistonite_cu as cu;
//! fn print_sources() -> cu::Result<()> {
//!     let mut builder = cu::fs::walker("src");
//!     builder.git(true).follow_links(true);
//!     builder.glob_includes(["**/*.{rs,toml}"].into_iter())?;
//!     for entry in builder.walk()? {
//!         let entry = entry?;
//!         cu::info!("{}", entry.path().display());
//!     }
//!     Ok(())
//! }
//! ```

use std::ffi::OsStr;
use std::fs::{FileType, Metadata};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use ignore::overrides::OverrideBuilder;
use ignore::{DirEntry as IgnoreDirEntry, Walk as IgnoreWalk, WalkBuilder as IgnoreWalkBuilder};

use crate::pre::*;

/// Create a [`WalkBuilder`] rooted at `root` to configure a walk.
///
/// Use this when you need to change the defaults (glob filters, gitignore,
/// following links, etc.); otherwise reach for [`walk`].
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// fn walk_dirs_too() -> cu::Result<()> {
///     let mut builder = cu::fs::walker(".");
///     builder.include_dir_entries(true);
///     for entry in builder.walk()? {
///         let entry = entry?;
///         cu::info!("{} (dir: {})", entry.path().display(), entry.is_dir());
///     }
///     Ok(())
/// }
/// ```
#[inline(always)]
pub fn walker(root: impl AsRef<Path>) -> WalkBuilder {
    WalkBuilder::new(root.as_ref().to_path_buf())
}

/// Recursively walk `root` with default settings.
///
/// The defaults are:
/// - includes hidden files,
/// - does not use `.gitignore` or other ignore files,
/// - does not follow symbolic links,
/// - does not apply any glob include/exclude filters,
/// - yields files only (directory entries are skipped).
///
/// Use [`walker`] to change any of these. The returned [`Walk`] is an iterator
/// of [`WalkEntry`] results.
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// fn count_files() -> cu::Result<usize> {
///     let mut count = 0;
///     for entry in cu::fs::walk(".")? {
///         let _entry = entry?;
///         count += 1;
///     }
///     Ok(count)
/// }
/// ```
#[inline(always)]
pub fn walk(root: impl AsRef<Path>) -> cu::Result<Walk> {
    walker(root).walk()
}

/// Builder for a directory [`Walk`], providing a simpler API over the
/// `ignore` crate for the most common cases.
///
/// Create one with [`walker`], configure it with the methods below, then call
/// [`walk`](WalkBuilder::walk) to produce the [`Walk`] iterator. Configuration
/// methods return `&mut Self` (or `cu::Result<&mut Self>` for the fallible glob
/// setters) so they can be chained.
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// fn configured() -> cu::Result<()> {
///     let mut builder = cu::fs::walker(".");
///     builder.ignore_hidden(true).include_dir_entries(true);
///     for entry in builder.walk()? {
///         let entry = entry?;
///         cu::info!("{}", entry.path().display());
///     }
///     Ok(())
/// }
/// ```
///
/// See [`walker`].
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

    /// Add glob patterns to include. By default all paths are included.
    ///
    /// Patterns use gitignore-style glob syntax (as implemented by the `ignore`
    /// crate), which supports `*`, `**`, `?`, `[...]` character classes, and
    /// `{a,b}` brace alternation. Once any include pattern is added, only paths
    /// matching at least one include (and no exclude) are yielded.
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// fn only_sources() -> cu::Result<()> {
    ///     let mut builder = cu::fs::walker(".");
    ///     // include Rust and TOML files anywhere in the tree
    ///     builder.glob_includes(["**/*.{rs,toml}"].into_iter())?;
    ///     for entry in builder.walk()? {
    ///         cu::info!("{}", entry?.path().display());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn glob_includes(
        &mut self,
        globs: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> crate::Result<&mut Self> {
        for g in globs {
            let g = g.as_ref();
            crate::check!(
                self.overrides.add(g),
                "failed to add glob include pattern: '{g}'"
            )?;
            self.has_overrides = true;
        }
        Ok(self)
    }

    /// Add glob patterns to exclude. By default nothing is excluded.
    ///
    /// Patterns use the same gitignore-style syntax as
    /// [`glob_includes`](Self::glob_includes), including `{a,b}` brace
    /// alternation. Excludes take precedence over includes.
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// fn skip_logs_and_tmp() -> cu::Result<()> {
    ///     let mut builder = cu::fs::walker(".");
    ///     builder.glob_excludes(["**/*.{log,tmp}"].into_iter())?;
    ///     for entry in builder.walk()? {
    ///         cu::info!("{}", entry?.path().display());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn glob_excludes(
        &mut self,
        globs: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> crate::Result<&mut Self> {
        let mut s = String::new();
        s.push('!');
        for g in globs {
            let g = g.as_ref();
            s.push_str(g);
            crate::check!(
                self.overrides.add(&s),
                "failed to add glob exclude pattern: '{s}'"
            )?;
            s.truncate(1);
            self.has_overrides = true;
        }
        Ok(self)
    }

    /// Set whether directory entries are returned while iterating. Default is
    /// `false` (only files are yielded).
    ///
    /// When enabled, directory entries and symlinks-to-directories are also
    /// yielded (as well as the root of the walk, at depth 0). Note that the
    /// files *inside* a symlinked directory are still not returned unless
    /// [`follow_links`](Self::follow_links) is also enabled.
    #[inline(always)]
    pub fn include_dir_entries(&mut self, include: bool) -> &mut Self {
        self.include_dir_entries = include;
        self
    }

    /// Enable reading `.gitignore` and git exclude configs, (mostly) matching
    /// git's own behavior. Default is disabled.
    ///
    /// Because this relies on git configuration, ignore rules are only applied
    /// when the walk root is inside a git repository.
    #[inline(always)]
    pub fn git(&mut self, yes: bool) -> &mut Self {
        self.inner.git_global(yes);
        self.inner.git_ignore(yes);
        self.inner.git_exclude(yes);
        self
    }

    /// Skip hidden files (dotfiles). Default is `false` (hidden files are
    /// included).
    #[inline(always)]
    pub fn ignore_hidden(&mut self, yes: bool) -> &mut Self {
        self.inner.hidden(yes);
        self
    }

    /// Follow symbolic links. Default is `false`.
    ///
    /// When enabled, symlinked directories are descended into. Following a
    /// dangling (broken) symlink surfaces an error from the [`Walk`] iterator
    /// rather than being silently skipped.
    pub fn follow_links(&mut self, yes: bool) -> &mut Self {
        self.inner.follow_links(yes);
        self
    }

    /// Add a custom ignore file name (in addition to any enabled by [`git`](Self::git)).
    ///
    /// Files with this name are read as gitignore-style ignore lists. Unlike
    /// [`git`](Self::git), custom ignore files are honored regardless of whether
    /// the walk root is inside a git repository.
    #[inline(always)]
    pub fn add_ignore_filename(&mut self, ignore_file: &str) -> &mut Self {
        self.inner.add_custom_ignore_filename(ignore_file);
        self
    }

    /// Build the [`Walk`] iterator from this configuration.
    ///
    /// Fails if any configured glob patterns cannot be compiled.
    pub fn walk(mut self) -> cu::Result<Walk> {
        if self.has_overrides {
            let overrides = cu::check!(
                self.overrides.build(),
                "walk: failed to build glob pattern overrides"
            )?;
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

/// An iterator over the entries of a directory walk.
///
/// Created by [`walk`] or [`WalkBuilder::walk`]. Each item is a
/// `cu::Result<WalkEntry>`; an `Err` indicates a failure reading a particular
/// entry (for example, following a broken symlink) and does not necessarily
/// stop the iteration.
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// fn list() -> cu::Result<()> {
///     for entry in cu::fs::walk(".")? {
///         let entry = entry?;
///         cu::info!("depth {}: {}", entry.depth(), entry.path().display());
///     }
///     Ok(())
/// }
/// ```
pub struct Walk {
    inner: IgnoreWalk,
    include_dir_entries: bool,
    root: Arc<PathBuf>,
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
                        crate::trace!(
                            "walk: skipping symlinked directory: '{}'",
                            entry.path().display()
                        );
                        continue;
                    }
                    crate::trace!(
                        "walk: skipping entry with unknown file type: '{}'",
                        entry.path().display()
                    );
                    continue;
                }
            }
        };
        Ok(Some(WalkEntry {
            root: Arc::clone(&self.root),
            inner: entry,
            file_type,
        }))
    }
}

/// A single entry produced by a [`Walk`].
///
/// Provides access to the entry's [`path`](Self::path), its
/// [`depth`](Self::depth) relative to the walk root, its
/// [`file_type`](Self::file_type), and lazily-read [`metadata`](Self::metadata).
#[derive(Debug)]
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

    /// Get this entry's path relative to the walk root, without a leading `./`.
    ///
    /// Returns `Ok(None)` when the entry *is* the root of the walk (only emitted
    /// when [`include_dir_entries`](WalkBuilder::include_dir_entries) is
    /// enabled). Returns an error if the entry path is unexpectedly not
    /// contained within the walk root.
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// fn print_relative() -> cu::Result<()> {
    ///     for entry in cu::fs::walk("src")? {
    ///         let entry = entry?;
    ///         if let Some(rel) = entry.rel_path()? {
    ///             cu::info!("{}", rel.display());
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn rel_path(&self) -> cu::Result<Option<PathBuf>> {
        // ensure root is a prefix of inner path
        let root_norm = self.root.normalize()?;
        // note we cannot normalize the path after join since it might be a symlink
        let path_norm = root_norm.join(self.inner.path());
        let root_iter = root_norm
            .components()
            .filter(|x| !matches!(x, Component::CurDir));
        let mut path_iter = path_norm
            .components()
            .filter(|x| !matches!(x, Component::CurDir));
        for root_comp in root_iter {
            let path_comp = cu::check!(
                path_iter.next(),
                "unexpected: walk entry path is shorter than root"
            )?;
            cu::ensure!(
                root_comp == path_comp,
                "unexpected: walk entry path is not in root"
            )?;
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
