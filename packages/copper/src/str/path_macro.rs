/// Efficient Path-join macro
///
/// **The macro rules above is only for illustration purpose, see source code for implementation**
///
/// ## Usage
/// The macro efficiently creates joined paths from either a owned `PathBuf`
/// or a borrowed Path reference (`impl AsRef<Path>`), and one or more path segments reference
/// to join. The OS separator is used (i.e. `\` on Windows).
///
/// The format of the macro in pseudocode is:
/// ```rust,ignore
/// cu::path!( FIRST_SEG  $( / NEXT_SEG )* )
/// ```
///
/// `FIRST_SEG` can be:
/// - A owned `PathBuf` ident
///   - e.g. `cu::path!(my_path_buf / ...)`
/// - A borrowed `&Path` ident:
///   - e.g. `cu::path!(&my_path / ...)`
///   - Here `&` is the macro rule to indicate you don't want to borrow the path, so you need
///     it even when `my_path` is already a borrowed path
/// - A literal string, which you can use without `&`
///   - e.g. `cu::path!("my_path" / ...)`
/// - An expression that evaluates to a owned `PathBuf`
///   - e.g. `cu::path!( (get_path()) / ...)`
///   - Expression needs to be parenthesized because `/` cannot follow an expression in macro
///     rules. `{ }` also works
/// - An expression that evaludates to a borrowed `&Path`
///   - e.g. `cu::path!( &(my.path) / ... )`
///   - Expression needs to be parenthesized because `/` cannot follow an expression in macro
///     rules. `{ }` also works
///   - Here `&` is the macro rule to indicate you don't want to borrow the path, so you need
///     it even when `my_path` is already a borrowed path
///
/// Each `NEXT_SEG` can be:
/// - A literal string
/// - An ident (the macro will not take ownership of the variable)
/// - An expression wrapped with either `( )` or `{ }`. The last expression doesn't need to be
///   wrapped
///
/// ## Examples
/// ```rust
/// # use pistonite_cu as cu;
/// use std::path::{Path, PathBuf};
///
/// // From a literal string
/// let p1 = cu::path!("home" / "user");
/// let p2 = cu::path!("home" / "user" / "docs");
/// assert_eq!(p1, PathBuf::from("home").join("user"));
/// assert_eq!(p2, PathBuf::from("home").join("user").join("docs"));
///
/// // From an owned PathBuf ident (base is moved)
/// let base = PathBuf::from("usr").join("local");
/// let p = cu::path!(base / "bin" / "tool");
/// assert_eq!(p, PathBuf::from("usr").join("local").join("bin").join("tool"));
///
/// // From a borrowed &Path ident (use `&` even if already a reference)
/// let base = PathBuf::from("etc");
/// let base_ref: &Path = base.as_path();
/// let p = cu::path!(&base_ref / "nginx" / "nginx.conf");
/// assert_eq!(p, PathBuf::from("etc").join("nginx").join("nginx.conf"));
///
/// // From an expression returning PathBuf (must be parenthesized)
/// let p = cu::path!((PathBuf::from("usr").join("local")) / "bin");
/// assert_eq!(p, PathBuf::from("usr").join("local").join("bin"));
///
/// // From an expression returning &Path (must be parenthesized, and needs `&`)
/// let owned = PathBuf::from("var");
/// let p = cu::path!(&(owned.as_path()) / "log");
/// assert_eq!(p, PathBuf::from("var").join("log"));
///
/// // NEXT_SEG can be an ident — not moved, still usable after
/// let dir = "subdir";
/// let file = "file.txt";
/// let p = cu::path!("root" / dir / file);
/// assert_eq!(p, PathBuf::from("root").join(dir).join(file));
/// let _ = (dir, file); // still accessible
///
/// // NEXT_SEG can be an expression (must be parenthesized)
/// let sub = String::from("sub");
/// let p = cu::path!("root" / (sub.as_str()) / "output.log");
/// assert_eq!(p, PathBuf::from("root").join("sub").join("output.log"));
/// ```
///
/// ## Implementation
/// Currently this uses the same implementation as the standard library (as of 1.95.0)
/// that does not do any probing to pre-allocate the path based on the input iterator.
/// Each segment is `.push()`-ed onto the initial buffer in a loop.
///
#[cfg(doc)]
#[macro_export]
macro_rules! path {
    ($(&)? ident / $(ident_or_literal_or_expr) / * ) => {};
    (literal / $(ident_or_literal_or_expr) / * ) => {};
    ($(&)? ( path_expression ) / $(ident_or_literal_or_expr) / *) => {};
}

#[cfg(not(doc))]
#[macro_export]
macro_rules! path {
    ($first:ident / $($rest_segs:tt)* ) => {{
        let mut x = $first;
        $crate::__path_internal!(x / $($rest_segs)*)
    }};
    ( & $first:ident / $($rest_segs:tt)* ) => {{
        let mut x = std::path::PathBuf::from($first.to_owned());
        $crate::__path_internal!(x / $($rest_segs)*)
    }};
    ($first:literal / $($rest_segs:tt)* ) => {{
        let mut x = std::path::PathBuf::from($first);
        $crate::__path_internal!(x / $($rest_segs)*)
    }};
    (( $first:expr ) / $($rest_segs:tt)* ) => {{
        let mut x: ::std::path::PathBuf = { $first };
        $crate::__path_internal!(x / $($rest_segs)*)
    }};
    ( & ( $first:expr ) / $($rest_segs:tt)* ) => {{
        let x: &::std::path::Path = {$first}.as_ref();
        let mut x = x.to_path_buf();
        $crate::__path_internal!(x / $($rest_segs)*)
    }};
    ( $first:block / $($rest_segs:tt)* ) => {{
        let mut x: ::std::path::PathBuf = $first;
        $crate::__path_internal!(x / $($rest_segs)*)
    }};
    ( & $first:block / $($rest_segs:tt)* ) => {{
        let x: &::std::path::Path = $first.as_ref();
        let mut x = x.to_path_buf();
        $crate::__path_internal!(x / $($rest_segs)*)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __path_internal {
    // Non-expression terminals (1 or 2 remaining)
    ($first:ident / $second:literal) => {{
        $first.push($second); $first
    }};
    ($first:ident / $second:ident) => {{
        let x: &::std::path::Path = $second.as_ref();
        $first.push(x); $first
    }};
    ($first:ident / $second:literal / $third:literal) => {{
        $first.push($second); $first.push($third); $first
    }};
    ($first:ident / $second:literal / $third:ident) => {{
        $first.push($second);
        let x: &::std::path::Path = $third.as_ref();
        $first.push(x); $first
    }};
    ($first:ident / $second:ident / $third:literal) => {{
        let x: &::std::path::Path = $second.as_ref();
        $first.push(x);
        $first.push($third); $first
    }};
    ($first:ident / $second:ident / $third:ident) => {{
        let x: &::std::path::Path = $second.as_ref();
        $first.push(x);
        let x: &::std::path::Path = $third.as_ref();
        $first.push(x); $first
    }};

    // non-terminal muchering (2 at a time)
    ($first:ident / $second:literal / $third:literal / $($rest_segs:tt)* ) => {{
        $first.push($second); $first.push($third);
        $crate::__path_internal!($first / $($rest_segs)* )
    }};
    ($first:ident / $second:literal / $third:ident / $($rest_segs:tt)* ) => {{
        $first.push($second);
        let x: &::std::path::Path = $third.as_ref();
        $first.push(x);
        $crate::__path_internal!($first / $($rest_segs)* )
    }};
    ($first:ident / $second:literal / ( $third:expr ) / $($rest_segs:tt)* ) => {{
        $first.push($second);
        let x: &::std::path::Path = {$third}.as_ref();
        $first.push(x);
        $crate::__path_internal!($first / $($rest_segs)* )
    }};
    ($first:ident / $second:literal / $third:block / $($rest_segs:tt)* ) => {{
        $first.push($second);
        let x: &::std::path::Path = $third.as_ref();
        $first.push(x);
        $crate::__path_internal!($first / $($rest_segs)* )
    }};
    ($first:ident / $second:ident / $third:literal / $($rest_segs:tt)* ) => {{
        let x: &::std::path::Path = $second.as_ref();
        $first.push(x);
        $first.push($third);
        $crate::__path_internal!($first / $($rest_segs)* )
    }};
    ($first:ident / $second:ident / $third:ident / $($rest_segs:tt)* ) => {{
        let x: &::std::path::Path = $second.as_ref();
        $first.push(x);
        let x: &::std::path::Path = $third.as_ref();
        $first.push(x);
        $crate::__path_internal!($first / $($rest_segs)* )
    }};
    ($first:ident / $second:ident / ( $third:expr ) / $($rest_segs:tt)* ) => {{
        let x: &::std::path::Path = $second.as_ref();
        $first.push(x);
        let x: &::std::path::Path = {$third}.as_ref();
        $first.push(x);
        $crate::__path_internal!($first / $($rest_segs)* )
    }};
    ($first:ident / $second:ident / $third:block / $($rest_segs:tt)* ) => {{
        let x: &::std::path::Path = $second.as_ref();
        $first.push(x);
        let x: &::std::path::Path = $third.as_ref();
        $first.push(x);
        $crate::__path_internal!($first / $($rest_segs)* )
    }};
    // expression muchering (1 at a time)
    ($first:ident / ( $second:expr ) / $($rest_segs:tt)* ) => {{
        let x: &::std::path::Path = {$second}.as_ref();
        $first.push(x);
        $crate::__path_internal!($first / $($rest_segs)* )
    }};
    ($first:ident / $second:block / $($rest_segs:tt)* ) => {{
        let x: &::std::path::Path = $second.as_ref();
        $first.push(x);
        $crate::__path_internal!($first / $($rest_segs)* )
    }};

    // expression muchering, must be after the non-expression rules
    // so the `/` doesn't get interpreted as an operator

    ($first:ident / $second:literal / $third:expr) => {{
        $first.push($second);
        let x: &::std::path::Path = {$third}.as_ref();
        $first.push(x); $first
    }};
    ($first:ident / $second:ident / $third:expr) => {{
        let x: &::std::path::Path = $second.as_ref();
        $first.push(x);
        let x: &::std::path::Path = {$third}.as_ref();
        $first.push(x); $first
    }};
    ($first:ident / $second:expr) => {{
        let x: &::std::path::Path = {$second}.as_ref();
        $first.push(x); $first
    }};

}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    fn long_expected() -> PathBuf {
        PathBuf::from("a")
            .join("b")
            .join("c")
            .join("d")
            .join("e")
            .join("f")
            .join("g")
            .join("h")
            .join("i")
            .join("j")
    }

    // literal first, mix of literal / ident / expr segments
    #[test]
    fn long_path_from_literal() {
        let c = "c";
        let e = String::from("e");
        let h = "h";
        let p = crate::path!("a" / "b" / (c) / "d" / (e.as_str()) / "f" / "g" / h / "i" / "j");
        assert_eq!(p, long_expected());
        let _ = (c, h); // idents still accessible
    }

    // owned PathBuf first, mix of ident / expr / literal segments
    #[test]
    fn long_path_from_owned() {
        let base = PathBuf::from("a");
        let b = "b";
        let d = String::from("d");
        let g = "g";
        let p = crate::path!(base / (b) / "c" / (d.as_str()) / "e" / "f" / g / "h" / "i" / "j");
        assert_eq!(p, long_expected());
        let _ = (b, g);
    }

    // borrowed &Path first, mix of expr / literal / ident segments
    #[test]
    fn long_path_from_borrowed() {
        let base = PathBuf::from("a");
        let base_ref: &Path = base.as_path();
        let c = String::from("c");
        let f = "f";
        let i = "i";
        let p = crate::path!(&base_ref / "b" / (c.as_str()) / "d" / "e" / f / "g" / "h" / i / "j");
        assert_eq!(p, long_expected());
        let _ = (f, i);
    }

    // expr-owned first, mix of literal / ident / expr segments
    #[test]
    fn long_path_from_expr_owned() {
        let d = "d";
        let e = String::from("e");
        let f = String::from("f");
        let p = crate::path!(
            (PathBuf::from("a")) / "b" / "c" / d / (&e) / (f.as_str()) / "g" / "h" / "i" / "j"
        );
        assert_eq!(p, long_expected());
        let _ = d;
    }

    // expr-borrowed first, mix of ident / expr / literal segments
    #[test]
    fn long_path_from_expr_borrowed() {
        let base = PathBuf::from("a");
        let b = "b";
        let e = String::from("e");
        let j = "j";
        let p = crate::path!(
            &(base.as_path()) / b / "c" / "d" / (e.as_str()) / "f" / "g" / "h" / "i" / j
        );
        assert_eq!(p, long_expected());
        let _ = (b, j);
    }

    // no two consecutive segments share a type: cycles ident / expr / literal throughout
    // 10 segments (even) — exercises even-count terminal arm
    #[test]
    fn long_path_no_consecutive_same_type_even() {
        let b = "b";
        let c = String::from("c");
        let e = "e";
        let f = String::from("f");
        let h = "h";
        let i = String::from("i");
        let p = crate::path!(
            "a" / b / (c.as_str()) / "d" / e / (f.as_str()) / "g" / h / (i.as_str()) / "j"
        );
        assert_eq!(p, long_expected());
        let _ = (b, e, h);
    }

    fn nine_expected() -> PathBuf {
        PathBuf::from("a")
            .join("b")
            .join("c")
            .join("d")
            .join("e")
            .join("f")
            .join("g")
            .join("h")
            .join("i")
    }

    // no two consecutive segments share a type: cycles literal / ident / expr throughout
    // 9 segments (odd) — exercises odd-count terminal arm
    #[test]
    fn long_path_no_consecutive_same_type_odd() {
        let b = "b";
        let c = String::from("c");
        let e = "e";
        let f = String::from("f");
        let h = "h";
        let i = String::from("i");
        let p =
            crate::path!("a" / b / (c.as_str()) / "d" / e / (f.as_str()) / "g" / h / i.as_str());
        assert_eq!(p, nine_expected());
        let _ = (b, e, h);
    }
}
