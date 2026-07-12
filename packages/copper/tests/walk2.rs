//! Fixture tests for [`cu::fs::walk2`].
//!
//! Each test builds a stub directory tree in its own uniquely-named temp
//! directory (under `CARGO_TARGET_TMPDIR`) so tests can run in parallel, and
//! tears it down with a drop guard. Symlink-dependent cases are `#[cfg(unix)]`
//! since Windows symlink creation requires elevated privileges.
#![cfg(feature = "fs")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pistonite_cu as cu;
use cu::pre::*;
use pistonite_cu::fs::walk2::{Walk, walk, walker};

/// A temp directory that is recursively removed when dropped.
struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // best-effort cleanup; ignore errors during teardown
        let _ = cu::fs::rec_remove(&self.root);
    }
}

impl Fixture {
    fn root(&self) -> &Path {
        &self.root
    }
}

/// Build the fixture tree in a fresh temp dir named after the calling test.
///
/// Tree:
/// ```text
/// root/
///   a.txt
///   b.log
///   .hidden.txt
///   .gitignore          # "b.log\nignored/\n"
///   .customignore       # "d.rs\n"
///   .git/               # empty marker so require_git(true) sees a repo
///   sub/
///     c.txt
///     d.rs
///     nested/e.txt
///   .hiddendir/f.txt
///   ignored/g.txt
/// ```
/// On unix, additionally:
/// ```text
///   link_to_file -> a.txt
///   link_to_dir  -> sub
/// ```
/// (A dangling link is created only by the broken-symlink test, since following
/// it is an error and would interfere with the other symlink cases.)
fn make_fixture(name: &str) -> cu::Result<Fixture> {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("walk2_{name}"));
    // start from a clean slate in case a previous run left something behind
    cu::fs::make_dir_empty(&root)?;
    cu::fs::write(root.join("a.txt"), "a")?;
    cu::fs::write(root.join("b.log"), "b")?;
    cu::fs::write(root.join(".hidden.txt"), "hidden")?;
    cu::fs::write(root.join(".gitignore"), "b.log\nignored/\n")?;
    cu::fs::write(root.join(".customignore"), "d.rs\n")?;
    cu::fs::make_dir(root.join(".git"))?;

    cu::fs::make_dir(root.join("sub"))?;
    cu::fs::write(root.join("sub/c.txt"), "c")?;
    cu::fs::write(root.join("sub/d.rs"), "d")?;
    cu::fs::make_dir(root.join("sub/nested"))?;
    cu::fs::write(root.join("sub/nested/e.txt"), "e")?;

    cu::fs::make_dir(root.join(".hiddendir"))?;
    cu::fs::write(root.join(".hiddendir/f.txt"), "f")?;

    cu::fs::make_dir(root.join("ignored"))?;
    cu::fs::write(root.join("ignored/g.txt"), "g")?;

    #[cfg(unix)]
    {
        make_symlink("a.txt", &root.join("link_to_file"))?;
        make_symlink("sub", &root.join("link_to_dir"))?;
    }

    Ok(Fixture { root })
}

/// Create a symlink at `link` pointing at `target`, with error context.
#[cfg(unix)]
fn make_symlink(target: &str, link: &Path) -> cu::Result<()> {
    cu::check!(
        std::os::unix::fs::symlink(target, link),
        "failed to create symlink '{}' -> '{target}'",
        link.display()
    )
}

/// Normalize a relative path to a forward-slash string for stable comparisons.
fn norm(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Drain a walk into the set of relative-path strings it yields.
///
/// The root entry (`rel_path() == None`, only emitted with dir entries enabled)
/// is represented as `"."`.
fn collect(w: Walk) -> cu::Result<BTreeSet<String>> {
    let mut set = BTreeSet::new();
    for entry in w {
        let entry = entry?;
        match entry.rel_path()? {
            Some(p) => {
                set.insert(norm(&p));
            }
            None => {
                set.insert(".".to_string());
            }
        }
    }
    Ok(set)
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// --- cross-platform cases (no symlinks) --------------------------------------

#[test]
fn default_returns_files_only() -> cu::Result<()> {
    let fx = make_fixture("default_returns_files_only")?;
    let got = collect(walk(fx.root())?)?;
    // Every regular file, hidden included; no directories, and (on unix) no
    // symlink entries leak in.
    let expected = set(&[
        "a.txt",
        "b.log",
        ".hidden.txt",
        ".gitignore",
        ".customignore",
        "sub/c.txt",
        "sub/d.rs",
        "sub/nested/e.txt",
        ".hiddendir/f.txt",
        "ignored/g.txt",
    ]);
    assert_eq!(got, expected);
    Ok(())
}

#[test]
fn include_dir_entries_true() -> cu::Result<()> {
    let fx = make_fixture("include_dir_entries_true")?;
    let mut b = walker(fx.root());
    b.include_dir_entries(true);
    let got = collect(b.walk()?)?;

    // directory entries now appear...
    for dir in ["sub", "sub/nested", ".hiddendir", "ignored"] {
        assert!(got.contains(dir), "expected dir entry {dir:?} in {got:?}");
    }
    // ...alongside the files...
    assert!(got.contains("sub/c.txt"));
    // ...and the root entry (rel_path == None).
    assert!(got.contains("."), "expected root entry in {got:?}");
    Ok(())
}

#[test]
fn ignore_hidden_true() -> cu::Result<()> {
    let fx = make_fixture("ignore_hidden_true")?;
    let mut b = walker(fx.root());
    b.ignore_hidden(true);
    let got = collect(b.walk()?)?;

    for hidden in [
        ".hidden.txt",
        ".gitignore",
        ".customignore",
        ".hiddendir/f.txt",
    ] {
        assert!(
            !got.contains(hidden),
            "hidden {hidden:?} should be excluded: {got:?}"
        );
    }
    assert!(got.contains("a.txt"));
    assert!(got.contains("sub/c.txt"));
    Ok(())
}

#[test]
fn glob_includes_txt() -> cu::Result<()> {
    let fx = make_fixture("glob_includes_txt")?;
    let mut b = walker(fx.root());
    b.glob_includes(["*.txt"].into_iter())?;
    let got = collect(b.walk()?)?;

    assert!(got.contains("a.txt"));
    assert!(got.contains("sub/c.txt"));
    assert!(got.contains("sub/nested/e.txt"));
    assert!(
        !got.contains("b.log"),
        "non-txt should be excluded: {got:?}"
    );
    assert!(
        !got.contains("sub/d.rs"),
        "non-txt should be excluded: {got:?}"
    );
    Ok(())
}

#[test]
fn glob_excludes_log() -> cu::Result<()> {
    let fx = make_fixture("glob_excludes_log")?;
    let mut b = walker(fx.root());
    b.glob_excludes(["*.log"].into_iter())?;
    let got = collect(b.walk()?)?;

    assert!(!got.contains("b.log"), "*.log should be excluded: {got:?}");
    assert!(got.contains("a.txt"));
    assert!(got.contains("sub/d.rs"));
    Ok(())
}

#[test]
fn glob_includes_brace_alternation() -> cu::Result<()> {
    let fx = make_fixture("glob_includes_brace_alternation")?;
    let mut b = walker(fx.root());
    // brace alternation: include both .txt and .log, but not .rs
    b.glob_includes(["*.{txt,log}"].into_iter())?;
    let got = collect(b.walk()?)?;

    assert!(got.contains("a.txt"), "txt should be included: {got:?}");
    assert!(got.contains("b.log"), "log should be included: {got:?}");
    assert!(got.contains("sub/c.txt"));
    assert!(!got.contains("sub/d.rs"), "rs should be excluded: {got:?}");
    Ok(())
}

#[test]
fn glob_includes_multiple_patterns() -> cu::Result<()> {
    let fx = make_fixture("glob_includes_multiple_patterns")?;
    let mut b = walker(fx.root());
    // multiple include patterns are OR-ed together
    b.glob_includes(["*.rs", "*.log"].into_iter())?;
    let got = collect(b.walk()?)?;

    assert!(got.contains("sub/d.rs"), "{got:?}");
    assert!(got.contains("b.log"), "{got:?}");
    assert!(!got.contains("a.txt"), "txt should be excluded: {got:?}");
    Ok(())
}

#[test]
fn glob_excludes_brace_alternation() -> cu::Result<()> {
    let fx = make_fixture("glob_excludes_brace_alternation")?;
    let mut b = walker(fx.root());
    // brace alternation in an exclude: drop both .log and .rs
    b.glob_excludes(["*.{log,rs}"].into_iter())?;
    let got = collect(b.walk()?)?;

    assert!(!got.contains("b.log"), "log should be excluded: {got:?}");
    assert!(!got.contains("sub/d.rs"), "rs should be excluded: {got:?}");
    assert!(got.contains("a.txt"), "txt should remain: {got:?}");
    assert!(got.contains("sub/c.txt"));
    Ok(())
}

#[test]
fn git_true_respects_gitignore() -> cu::Result<()> {
    let fx = make_fixture("git_true_respects_gitignore")?;
    let mut b = walker(fx.root());
    b.git(true);
    let got = collect(b.walk()?)?;

    assert!(
        !got.contains("b.log"),
        "gitignored file should be excluded: {got:?}"
    );
    assert!(
        !got.contains("ignored/g.txt"),
        "gitignored dir contents should be excluded: {got:?}"
    );
    assert!(got.contains("a.txt"));
    assert!(got.contains("sub/c.txt"));
    Ok(())
}

#[test]
fn custom_ignore_filename() -> cu::Result<()> {
    let fx = make_fixture("custom_ignore_filename")?;
    let mut b = walker(fx.root());
    b.add_ignore_filename(".customignore");
    let got = collect(b.walk()?)?;

    assert!(
        !got.contains("sub/d.rs"),
        "custom-ignored file should be excluded: {got:?}"
    );
    assert!(got.contains("a.txt"));
    assert!(got.contains("sub/c.txt"));
    Ok(())
}

#[test]
fn entry_metadata_depth_relpath() -> cu::Result<()> {
    let fx = make_fixture("entry_metadata_depth_relpath")?;
    let mut b = walker(fx.root());
    b.include_dir_entries(true);

    let mut saw_root = false;
    let mut saw_top_file = false;
    let mut saw_nested_file = false;
    let mut saw_dir = false;

    for entry in b.walk()? {
        let entry = entry?;
        match entry.rel_path()? {
            None => {
                // the root entry
                assert_eq!(entry.depth(), 0, "root depth should be 0");
                assert!(entry.is_dir());
                saw_root = true;
            }
            Some(rel) => {
                let rel = norm(&rel);
                // rel_path must never carry a leading "./"
                assert!(!rel.starts_with("./"), "rel_path has leading ./: {rel}");
                match rel.as_str() {
                    "a.txt" => {
                        assert_eq!(entry.depth(), 1);
                        assert!(entry.is_file());
                        assert_eq!(entry.file_name().unwrap(), "a.txt");
                        entry.metadata()?;
                        saw_top_file = true;
                    }
                    "sub/nested/e.txt" => {
                        assert_eq!(entry.depth(), 3, "nested file should be depth 3");
                        assert!(entry.is_file());
                        saw_nested_file = true;
                    }
                    "sub" => {
                        assert_eq!(entry.depth(), 1);
                        assert!(entry.is_dir());
                        saw_dir = true;
                    }
                    _ => {}
                }
            }
        }
    }

    assert!(saw_root, "did not observe root entry");
    assert!(saw_top_file, "did not observe a.txt");
    assert!(saw_nested_file, "did not observe sub/nested/e.txt");
    assert!(saw_dir, "did not observe sub dir");
    Ok(())
}

// --- unix-only cases (symlinks) ----------------------------------------------

#[cfg(unix)]
#[test]
fn symlink_to_file_skipped_by_default() -> cu::Result<()> {
    let fx = make_fixture("symlink_to_file_skipped_by_default")?;
    let got = collect(walk(fx.root())?)?;

    // symlink-to-file is not a plain file, so it is skipped by default
    assert!(
        !got.contains("link_to_file"),
        "symlink to file should be skipped: {got:?}"
    );
    // symlinked directory is not descended (no follow)
    assert!(
        !got.contains("link_to_dir"),
        "symlinked dir should be skipped: {got:?}"
    );
    assert!(
        !got.iter().any(|p| p.starts_with("link_to_dir/")),
        "symlinked dir must not be descended: {got:?}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_entries_with_include_dir_entries() -> cu::Result<()> {
    let fx = make_fixture("symlink_entries_with_include_dir_entries")?;
    let mut b = walker(fx.root());
    b.include_dir_entries(true);
    let got = collect(b.walk()?)?;

    // links surface as entries when dir entries are included...
    assert!(
        got.contains("link_to_file"),
        "expected link_to_file entry: {got:?}"
    );
    assert!(
        got.contains("link_to_dir"),
        "expected link_to_dir entry: {got:?}"
    );
    // ...but the symlinked dir is still not descended without follow_links.
    assert!(
        !got.iter().any(|p| p.starts_with("link_to_dir/")),
        "symlinked dir must not be descended without follow_links: {got:?}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn follow_links_descends_symlinked_dir() -> cu::Result<()> {
    let fx = make_fixture("follow_links_descends_symlinked_dir")?;
    let mut b = walker(fx.root());
    b.follow_links(true);
    let got = collect(b.walk()?)?;

    // following the link exposes the target dir's files under the link name
    assert!(
        got.contains("link_to_dir/c.txt"),
        "follow_links should descend symlinked dir: {got:?}"
    );
    assert!(got.contains("link_to_dir/nested/e.txt"), "{got:?}");
    Ok(())
}

#[cfg(unix)]
#[test]
fn broken_symlink_without_follow_does_not_error() -> cu::Result<()> {
    let fx = make_fixture("broken_symlink_without_follow_does_not_error")?;
    make_symlink("does_not_exist", &fx.root().join("link_broken"))?;

    // Without following, the entry is read via symlink_metadata, so a dangling
    // target is fine: the walk completes without yielding an error.
    let mut b = walker(fx.root());
    b.include_dir_entries(true);
    let got = collect(b.walk()?)?;
    assert!(
        got.contains("link_broken"),
        "broken link should still surface as an entry: {got:?}"
    );

    // default walk (dir entries off) skips it as a non-file, also without error.
    let files = collect(walk(fx.root())?)?;
    assert!(
        !files.contains("link_broken"),
        "broken link is not a file: {files:?}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn broken_symlink_with_follow_surfaces_error() -> cu::Result<()> {
    let fx = make_fixture("broken_symlink_with_follow_surfaces_error")?;
    make_symlink("does_not_exist", &fx.root().join("link_broken"))?;

    // With follow_links, resolving the dangling target fails, and the walk
    // reports it as an error rather than silently skipping.
    let mut b = walker(fx.root());
    b.include_dir_entries(true).follow_links(true);
    let saw_error = b.walk()?.any(|entry| entry.is_err());
    assert!(
        saw_error,
        "following a dangling symlink should surface an error"
    );
    Ok(())
}
