/// `serde` JSON integration via the `serde_json` crate
///
/// Since `serde` derives requires linking with `serde` crate,
/// you have to add it to your `Cargo.toml`:
/// ```toml
/// [dependencies]
/// serde = "1"
/// ```
///
/// ```rust
/// # use pistonite_cu as cu;
/// use cu::pre::*; // with `serde` feature, Serialize and Deserialize are
///                 // part of the prelude
///
/// #[derive(Debug, PartialEq, Serialize, Deserialize)]
/// struct FooBar {
///     foo: String,
/// }
///
/// # fn main() -> cu::Result<()> {
/// // the json macro and json module will also be part of the prelude
/// let value1 = json!({"foo":"bar"});
/// let value2: FooBar = json::from_value(value1.clone())?;
/// assert_eq!(value2.foo, "bar");
///
/// let value3: FooBar = json::parse(r#"{ "foo": "bar" }"#)?;
/// assert_eq!(value2, value3);
///
/// # Ok(()) }
/// ```
pub mod json {
    use crate::{Context, Parse};
    #[cfg(feature = "parse-impl")]
    pub use ::serde_json::{de, error, map, ser, value};
    use serde::{Deserialize, Serialize};
    pub use serde_json::{Map, Number, Value};

    /// JSON parse delegate. See [`json`](module@crate::json)
    pub struct Json<T>(std::marker::PhantomData<T>);
    impl<T: for<'a> Deserialize<'a>> Parse for Json<T> {
        type Output = T;

        fn parse_borrowed(x: &str) -> crate::Result<Self::Output> {
            parse(x)
        }

        fn parse_read(x: impl std::io::Read) -> crate::Result<Self::Output> {
            serde_json::from_reader(x).with_context(|| {
                format!(
                    "failed to parse input as json into {}",
                    std::any::type_name::<T>()
                )
            })
        }
    }

    /// Parse value from a JSON `&str`
    #[inline(always)]
    pub fn parse<'a, T: Deserialize<'a>>(x: &'a str) -> crate::Result<T> {
        serde_json::from_str(x).with_context(|| {
            format!(
                "failed to parse input as json into {}",
                std::any::type_name::<T>()
            )
        })
    }

    /// Parse value from a reader that yields JSON
    #[inline(always)]
    pub fn read<T: for<'a> Deserialize<'a>>(x: impl std::io::Read) -> crate::Result<T> {
        Json::<T>::parse_read(x)
    }

    /// Parse value from an async reader that yields JSON
    #[cfg(feature = "coroutine")]
    pub async fn co_read<T: for<'a> Deserialize<'a>>(
        x: impl tokio::io::AsyncRead + Unpin,
    ) -> crate::Result<T> {
        crate::co_read::<Json<T>>(x).await
    }

    /// Convert a JSON value into T
    pub fn from_value<T: for<'a> Deserialize<'a>>(x: Value) -> crate::Result<T> {
        use crate::Context;
        crate::check!(
            serde_json::from_value(x),
            "failed to convert json value to {}",
            std::any::type_name::<T>()
        )
    }

    /// Convert a T into a JSON value
    pub fn to_value<T: ?Sized + Serialize>(x: &T) -> crate::Result<Value> {
        use crate::Context;
        crate::check!(
            serde_json::to_value(x),
            "failed to convert {} to json value",
            std::any::type_name::<T>()
        )
    }

    /// Convert the value to a JSON string
    pub fn stringify<T: ?Sized + Serialize>(x: &T) -> crate::Result<String> {
        use crate::Context;
        crate::check!(
            serde_json::to_string(x),
            "failed to serialize {} to json",
            std::any::type_name::<T>()
        )
    }

    /// Convert the value to a pretty-formatted JSON string
    pub fn stringify_pretty<T: ?Sized + Serialize>(x: &T) -> crate::Result<String> {
        use crate::Context;
        crate::check!(
            serde_json::to_string_pretty(x),
            "failed to serialize {} to json",
            std::any::type_name::<T>()
        )
    }

    /// Serialize the value into JSON and write into the writer
    pub fn write<T: ?Sized + Serialize>(w: impl std::io::Write, x: &T) -> crate::Result<()> {
        use crate::Context;
        crate::check!(
            serde_json::to_writer(w, x),
            "failed to write serialized {} as json",
            std::any::type_name::<T>()
        )
    }

    /// Serialize the value into pretty-formatted JSON and write into the writer
    pub fn write_pretty<T: ?Sized + Serialize>(w: impl std::io::Write, x: &T) -> crate::Result<()> {
        use crate::Context;
        crate::check!(
            serde_json::to_writer_pretty(w, x),
            "failed to write serialized {} as json",
            std::any::type_name::<T>()
        )
    }
}
#[cfg_attr(any(docsrs, feature = "nightly"), doc(cfg(feature = "json")))]
pub use serde_json::json;
