/// `serde` TOML integration via the `toml` crate
///
/// ```rust
/// use cu::pre::*; // with `serde` feature, Serialize and Deserialize are
///                 // part of the prelude
///
/// use std::collections::BTreeMap;
///
/// #[derive(Debug, PartialEq, Serialize, Deserialize)]
/// struct FooBar {
///     foo: BTreeMap<String, String>,
/// }
///
/// # fn main() -> cu::Result<()> {
/// // the toml macro and toml module will also be part of the prelude
/// let value1 = toml! {
///     [foo]
///     bar = "biz"
/// };
/// assert_eq!(value1["foo"]["bar"], "biz".into());
/// let value2: FooBar = toml::parse(r#"
/// [foo]
/// bar = "biz"
/// "#)?;
/// assert_eq!(
///     Some("biz"),
///     value2.foo.get("bar").map(|x| x.as_str()),
/// );
///
/// # Ok(()) }
/// ```
pub mod toml {
    use crate::{Context, Parse};
    pub use ::toml::value::{Date, Datetime, Offset, Time};
    pub use ::toml::{Spanned, Table, Value, value::Array as Vec};
    use serde::{Deserialize, Serialize};

    /// TOML parse delegate. See [`toml`](module@crate::toml)
    pub struct Toml<T>(std::marker::PhantomData<T>);
    impl<T: for<'a> Deserialize<'a>> Parse for Toml<T> {
        type Output = T;

        fn parse_borrowed(x: &str) -> crate::Result<Self::Output> {
            ::toml::from_str(x).with_context(|| {
                format!(
                    "failed to parse input as toml into {}",
                    std::any::type_name::<T>()
                )
            })
        }
    }

    /// Parse value from a TOML `&str`
    #[inline(always)]
    pub fn parse<T: for<'a> Deserialize<'a>>(x: &str) -> crate::Result<T> {
        Toml::<T>::parse_borrowed(x)
    }

    /// Parse value from a reader that yields TOML
    #[inline(always)]
    pub fn read<T: for<'a> Deserialize<'a>>(x: impl std::io::Read) -> crate::Result<T> {
        Toml::<T>::parse_read(x)
    }

    /// Parse value from an async reader that yields TOML
    #[cfg(feature = "coroutine")]
    pub async fn co_read<T: for<'a> Deserialize<'a>>(
        x: impl tokio::io::AsyncRead + Unpin,
    ) -> crate::Result<T> {
        crate::co_read::<Toml<T>>(x).await
    }

    /// Convert the value to a TOML string
    pub fn stringify<T: ?Sized + Serialize>(x: &T) -> crate::Result<String> {
        use crate::Context;
        crate::check!(
            toml::to_string(x),
            "failed to serialize {} to toml",
            std::any::type_name::<T>()
        )
    }

    /// Convert the value to a pretty-formatted TOML string
    pub fn stringify_pretty<T: ?Sized + Serialize>(x: &T) -> crate::Result<String> {
        use crate::Context;
        crate::check!(
            toml::to_string_pretty(x),
            "failed to serialize {} to toml",
            std::any::type_name::<T>()
        )
    }
}
#[cfg_attr(any(docsrs, feature = "nightly"), doc(cfg(feature = "toml")))]
pub use ::toml::toml;
