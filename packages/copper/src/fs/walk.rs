use std::ffi::OsString;
use std::fs::{FileType, Metadata};
use std::path::{Path, PathBuf};

use crate::pre::*;

/// Recursively walk a directory
pub fn walk(path: impl AsRef<Path>) -> crate::Result<Walk> {
    walk_with(path, AlwaysRecurse)
}

pub fn walk_with<F>(path: impl AsRef<Path>, should_recurse: F) -> crate::Result<Walk<F>>
where
    F: for<'a> WalkShouldRecursePredicate<WalkEntry<'a>>,
{
    let path = path.as_ref().to_path_buf();
    crate::trace!("walk '{}'", path.display());
    let reader = crate::check!(
        std::fs::read_dir(&path),
        "cannot read directory '{}'",
        path.display()
    )?;
    // reserve with initial capacity
    #[allow(clippy::vec_init_then_push)]
    let mut stack = Vec::with_capacity(4);
    stack.push(reader);
    Ok(Walk {
        root: path,
        rel_containing: PathBuf::new(),
        stack,
        should_recurse,
    })
}
pub struct Walk<F = AlwaysRecurse> {
    root: PathBuf,
    /// The path of the containing directory
    /// of the current entry, relative from the root of the walk
    rel_containing: PathBuf,
    /// last element of the stack is the current directory being read
    stack: Vec<std::fs::ReadDir>,

    should_recurse: F,
}

impl<F> Walk<F>
where
    F: for<'b> WalkShouldRecursePredicate<WalkEntry<'b>>,
{
    #[allow(clippy::should_implement_trait)]
    // ^ iterator does not allow returning items referencing data from the iterator
    pub fn next(&mut self) -> Option<crate::Result<WalkEntry<'_>>> {
        loop {
            let dir = self.stack.last_mut()?;
            // find next item in the current dir
            let entry = match dir.next() {
                None => {
                    // current directory is done, go back to parent
                    self.stack.pop();
                    self.rel_containing.pop();
                    continue;
                }
                Some(Err(e)) => {
                    return Some(Err(e).context(format!(
                        "failed to read directory entry while walking '{}'",
                        self.root.display()
                    )));
                }
                Some(Ok(entry)) => entry,
            };
            let file_type = match entry.file_type() {
                Err(e) => {
                    return Some(Err(e).context(format!(
                        "failed to read directory entry type while walking '{}'",
                        self.root.display()
                    )));
                }
                Ok(x) => x,
            };
            let file_name = entry.file_name();
            if file_type.is_dir() {
                let entry = WalkEntry {
                    root: &self.root,
                    file_type,
                    rel_containing: &self.rel_containing,
                    file_name,
                    entry,
                };
                if !self.should_recurse.should_recurse(&entry) {
                    continue;
                }
                // enter the directory
                self.rel_containing.push(entry.file_name);
                let dir = self.root.join(&self.rel_containing);
                let read_dir = match std::fs::read_dir(dir) {
                    Err(e) => {
                        let rel_containing2 = self.rel_containing.display().to_string();
                        self.rel_containing.pop();
                        return Some(Err(e).context(format!(
                            "failed to read nested directory '{}' while walking '{}'",
                            rel_containing2,
                            self.root.display()
                        )));
                    }
                    Ok(read_dir) => read_dir,
                };

                self.stack.push(read_dir);
                continue;
            }
            let entry = WalkEntry {
                root: &self.root,
                file_type,
                rel_containing: &self.rel_containing,
                file_name,
                entry,
            };
            return Some(Ok(entry));
        }
    }
}

pub struct WalkEntry<'a> {
    /// Root path of the walk
    pub root: &'a Path,
    /// Type of the entry
    pub file_type: FileType,
    /// The directory that contains the current entry, relative
    /// to the root where the walk started, without the leading `./`.
    pub rel_containing: &'a Path,
    /// File name of the current entry being visited
    pub file_name: OsString,
    entry: std::fs::DirEntry,
}
impl WalkEntry<'_> {
    /// Get the path by joining the walk root and the relative
    /// path of the entry
    pub fn path(&self) -> PathBuf {
        self.entry.path()
    }

    /// Get the relative path of this entry, from the walk root
    pub fn rel_path(&self) -> PathBuf {
        self.rel_containing.join(&self.file_name)
    }

    /// Get the entry metadata
    pub fn metadata(&self) -> crate::Result<Metadata> {
        crate::check!(
            self.entry.metadata(),
            "failed to get metadata for file '{}' while walking directory '{}'",
            self.rel_path().display(),
            self.root.display()
        )
    }
}

pub trait WalkShouldRecursePredicate<E> {
    fn should_recurse(&mut self, entry: &E) -> bool;
}
pub struct AlwaysRecurse;
impl<E> WalkShouldRecursePredicate<E> for AlwaysRecurse {
    fn should_recurse(&mut self, _: &E) -> bool {
        true
    }
}
impl<'a, F> WalkShouldRecursePredicate<WalkEntry<'a>> for F
where
    F: for<'b> Fn(&WalkEntry<'b>) -> bool,
{
    fn should_recurse(&mut self, entry: &WalkEntry<'a>) -> bool {
        (self)(entry)
    }
}
