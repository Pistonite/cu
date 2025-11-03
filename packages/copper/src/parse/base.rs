/// The parsing utilities in `cu` provides wrapper to the std library
/// for parsing common data, such as different radix for numbers,
/// and support for common formats like `json`, `yaml` and `toml`.
///
/// Everything only works with UTF-8.
///
/// The entry points to parsing are [`cu::parse::<T>`](crate::parse), and `"...".parse_to::<T>()`.
/// Use whichever is more readable or preferrable at the call sites.
///
/// ```rust
/// # use pistonite_cu as cu;
/// use std::path::PathBuf;
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// # use pistonite_cu as cu;
/// // bool: empty string, 0, 1, "true", or "false", case-insensitive
/// let x = cu::parse::<bool>("0")?;
/// assert_eq!(x, false);
///
/// // numbers: hex, oct and binary are supported with 0x, 0b, and 0o prefix
/// let x = cu::parse::<u32>("0")?;
/// assert_eq!(x, 0);
/// let x = cu::parse::<i32>("-123")?;
/// assert_eq!(x, -123);
/// let x = cu::parse::<usize>("0x123")?;
/// assert_eq!(x, 0x123);
///
/// // .parse_to() can be used when chaining,
/// // instead of wrapping in cu::parse or use a temporary variable.
/// let _x = "imagine some long chaining is used to get this value"
///     .parse_to::<PathBuf>()?;
/// # Ok(()) }
/// ```
///
/// If the value is owned, you can use `parse_owned` instead of `parse`.
/// For readers, use [`cu::read`] or [`cu::co_read`] for async readers.
///
/// # From Owned and Read
/// When possible, implementation of parsing from a `io::Read` is provided
/// for optimization (for example for `json` inputs using `serde_json::from_reader`).
///
/// Async read is also supported, however, at the time of writing,
/// the inputs are first read into a buffer asynchronously, then
/// parsed synchronously from the buffer.
///
/// # Remote Derive
/// Common types are implemented in this library. You can derive
/// this trait by using the [`Parse`] derive macro
/// for any type that implements [`FromStr`].
///
/// # Common Formats
/// When `json`/`yaml`/`toml` features are enabled, the respective
/// module will be included in the prelude.
/// See [`json`]/[`yaml`]/[`toml`] for more details.
///
/// [`FromStr`]: std::str::FromStr
/// [`Parse`]: macro@crate::Parse
/// [`json`]: module@crate::json
/// [`yaml`]: module@crate::yaml
/// [`toml`]: module@crate::toml
/// [`cu::read`]: crate::read
/// [`cu::co_read`]: crate::co_read
pub trait Parse
where
    Self: Sized,
{
    type Output;
    fn parse_borrowed(x: &str) -> crate::Result<Self::Output>;
    fn parse_owned(x: String) -> crate::Result<Self::Output> {
        Self::parse_borrowed(&x)
    }
    fn parse_read(mut x: impl std::io::Read) -> crate::Result<Self::Output> {
        use crate::Context as _;
        let mut s = String::new();
        x.read_to_string(&mut s).context("io error while parsing")?;
        Self::parse_owned(s)
    }
}

/// the `.parse_to` function.
///
/// See [`Parse`](trait@Parse)
pub trait ParseTo {
    fn parse_to<T: Parse>(self) -> crate::Result<T::Output>;
}
impl ParseTo for &str {
    fn parse_to<T: Parse>(self) -> crate::Result<T::Output> {
        T::parse_borrowed(self)
    }
}
impl ParseTo for &mut str {
    fn parse_to<T: Parse>(self) -> crate::Result<T::Output> {
        T::parse_borrowed(self)
    }
}
impl ParseTo for String {
    fn parse_to<T: Parse>(self) -> crate::Result<T::Output> {
        T::parse_owned(self)
    }
}

/// Parse a value from a `&str`.
///
/// See [`Parse`](trait@Parse)
#[inline(always)]
pub fn parse<T: Parse>(x: &str) -> crate::Result<T::Output> {
    T::parse_borrowed(x)
}

/// Parse a value from a `String`.
///
/// See [`Parse`](trait@Parse)
#[inline(always)]
pub fn parse_owned<T: Parse>(x: String) -> crate::Result<T::Output> {
    T::parse_owned(x)
}

/// Parse a value from a `std::io::Read`.
///
/// See [`Parse`](trait@Parse)
#[inline(always)]
pub fn read<T: Parse>(x: impl std::io::Read) -> crate::Result<T::Output> {
    T::parse_read(x)
}

/// Parse a value from a `tokio::io::AsyncRead`.
///
/// See [`Parse`](trait@Parse)
#[cfg(feature = "coroutine")]
pub async fn co_read<T: Parse>(
    mut x: impl tokio::io::AsyncRead + Unpin,
) -> crate::Result<T::Output> {
    use crate::Context as _;
    use tokio::io::AsyncReadExt as _;
    let mut s = String::new();
    x.read_to_string(&mut s)
        .await
        .context("io error while parsing")?;
    T::parse_owned(s)
}
