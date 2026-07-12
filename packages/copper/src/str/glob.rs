//! Glob patterns with brace expansion.
//!
//! [`GlobPattern`] wraps rust-lang's `glob` crate to add support for
//! brace alternation (`{a,b}`), which `glob::Pattern` does not implement.
//! The input is expanded into every permutation it describes, and each
//! permutation is compiled into its own `glob::Pattern`. A string matches
//! the [`GlobPattern`] if it matches any of them.
//!
//! Expansion is capped at 4096 permutations, since alternations multiply.

use std::path::Path;

use ::glob::Pattern;
use smallvec::SmallVec;

/// Maximum number of patterns a single input may expand into.
const MAX_PERMUTATIONS: usize = 4096;

/// A glob pattern that additionally supports brace alternation (`{a,b}`).
///
/// This is a wrapper for rust-lang's `glob` crate. All of the usual glob
/// syntax is supported (`*`, `**`, `?`, and `[...]` character classes),
/// plus `{a,b}` segments, which mean "either `a` or `b`".
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// let p = cu::str::GlobPattern::new("**/*.{rs,toml}").unwrap();
/// assert!(p.matches("src/main.rs"));
/// assert!(p.matches("src/Cargo.toml"));
/// assert!(!p.matches("src/README.md"));
/// ```
#[cfg_attr(any(docsrs, feature = "nightly"), doc(cfg(feature = "fs")))]
#[derive(Debug, Clone)]
pub struct GlobPattern(SmallVec<[Pattern; 4]>);

impl GlobPattern {
    /// Compile a glob pattern, expanding any `{a,b}` alternations.
    ///
    /// Alternations may be nested (`{a,{b,c}}`) and combined
    /// (`{a,b}/{c,d}` describes 4 patterns). A single alternative
    /// (`{a}`) is allowed and simply means `a`.
    ///
    /// Note that `glob` has no escape character. To match a literal `{`,
    /// wrap it in a character class: `[{]`. Braces and commas inside
    /// `[...]` are always literal and are never expanded.
    ///
    /// # Errors
    /// Malformed alternations are rejected rather than being silently
    /// treated as literal text, so that typos surface instead of quietly
    /// matching nothing:
    /// - an unmatched `{` or `}`, as in `a{b` or `a}b`
    /// - an empty alternative, as in `{a,}`, `{,a}`, or `{}`
    ///
    /// This also errors if the pattern expands to more than 4096
    /// permutations, or if any expanded permutation is not a valid glob.
    pub fn new(pattern: &str) -> crate::Result<Self> {
        let _ = pattern;
        todo!("brace expansion + Pattern compilation")
    }

    /// Check if the string matches any of the expanded patterns.
    pub fn matches(&self, s: &str) -> bool {
        let _ = s;
        todo!()
    }

    /// Check if the path matches any of the expanded patterns.
    pub fn matches_path(&self, path: &Path) -> bool {
        let _ = path;
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::GlobPattern;

    fn pattern(p: &str) -> GlobPattern {
        GlobPattern::new(p).expect("pattern should compile")
    }

    #[test]
    fn no_braces() {
        let p = pattern("**/*.rs");
        assert!(p.matches("src/main.rs"));
        assert!(!p.matches("src/README.md"));
    }

    #[test]
    fn alternation() {
        let p = pattern("**/*.{rs,toml}");
        assert!(p.matches("src/main.rs"));
        assert!(p.matches("src/Cargo.toml"));
        assert!(!p.matches("src/README.md"));
    }

    #[test]
    fn cross_product() {
        let p = pattern("{a,b}/{c,d}");
        assert!(p.matches("a/c"));
        assert!(p.matches("a/d"));
        assert!(p.matches("b/c"));
        assert!(p.matches("b/d"));
        assert!(!p.matches("a/b"));
        assert!(!p.matches("c/a"));
    }

    #[test]
    fn nested() {
        let p = pattern("{a,{b,c}}");
        assert!(p.matches("a"));
        assert!(p.matches("b"));
        assert!(p.matches("c"));
        assert!(!p.matches("d"));
    }

    #[test]
    fn single_alternative() {
        let p = pattern("{a}");
        assert!(p.matches("a"));
        assert!(!p.matches("{a}"));
    }

    #[test]
    fn bracket_literal_brace() {
        let p = pattern("[{]");
        assert!(p.matches("{"));
        assert!(!p.matches("["));
    }

    #[test]
    fn bracket_comma() {
        // a character class, not an alternation
        let p = pattern("[a,b]");
        assert!(p.matches("a"));
        assert!(p.matches(","));
        assert!(p.matches("b"));
        assert!(!p.matches("a,b"));
    }

    #[test]
    fn bracket_negated_comma() {
        let p = pattern("[!,]");
        assert!(p.matches("a"));
        assert!(!p.matches(","));
    }

    #[test]
    fn bracket_close_first() {
        // a leading `]` in a character class is literal
        let p = pattern("[]]");
        assert!(p.matches("]"));

        let p = pattern("[!]]");
        assert!(p.matches("a"));
        assert!(!p.matches("]"));
    }

    #[test]
    fn err_unmatched_open() {
        assert!(GlobPattern::new("a{b").is_err());
    }

    #[test]
    fn err_unmatched_close() {
        assert!(GlobPattern::new("a}b").is_err());
        assert!(GlobPattern::new("{a,b}}").is_err());
    }

    #[test]
    fn err_empty_alternative() {
        assert!(GlobPattern::new("{a,}").is_err());
        assert!(GlobPattern::new("{,a}").is_err());
        assert!(GlobPattern::new("{}").is_err());
    }

    #[test]
    fn cap_at_boundary() {
        // 2^12 == 4096, exactly at the cap
        assert!(GlobPattern::new(&"{a,b}".repeat(12)).is_ok());
        // 2^13 == 8192, over the cap
        assert!(GlobPattern::new(&"{a,b}".repeat(13)).is_err());
    }

    #[test]
    fn deep_nesting_does_not_overflow() {
        // A comma-less alternation adds no permutations, so the cap never
        // fires. A recursive expander would exhaust the stack here; we only
        // care that this returns at all, not what it returns.
        let _ = GlobPattern::new(&"{a}".repeat(10_000));
    }

    #[test]
    fn matches_path_agrees_with_matches() {
        let p = pattern("**/*.{rs,toml}");
        assert!(p.matches_path(Path::new("src/main.rs")));
        assert!(!p.matches_path(Path::new("src/README.md")));
    }
}
