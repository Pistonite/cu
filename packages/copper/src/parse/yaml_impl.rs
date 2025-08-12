/// `serde` YAML integration via the `serde_yaml_ng` crate
///
/// ```rust
/// use cu::pre::*; // with `serde` feature, Serialize and Deserialize are
///                 // part of the prelude
///
/// #[derive(Debug, PartialEq, Serialize, Deserialize)]
/// struct FooBar {
///     foo: String,
/// }
///
/// # fn main() -> cu::Result<()> {
/// // the yaml module will also be part of the prelude
///
/// let value: FooBar = yaml::parse(r#"foo: bar"#)?;
/// assert_eq!(value.foo, "bar");
///
/// # Ok(()) }
/// ```
///
pub mod yaml {
    use crate::{Context, Parse};
    use serde::{Deserialize, Serialize};
    pub use serde_yaml_ng::{Mapping as Map, Number, Sequence as Vec, Value};

    /// YAML parse delegate. See [`yaml`](module@crate::yaml)
    pub struct Yaml<T>(std::marker::PhantomData<T>);
    impl<T: for<'a> Deserialize<'a>> Parse for Yaml<T> {
        type Output = T;

        fn parse_borrowed(x: &str) -> crate::Result<Self::Output> {
            serde_yaml_ng::from_str(x).with_context(|| {
                format!(
                    "failed to parse input as yaml into {}",
                    std::any::type_name::<T>()
                )
            })
        }

        fn parse_read(x: impl std::io::Read) -> crate::Result<Self::Output> {
            serde_yaml_ng::from_reader(x).with_context(|| {
                format!(
                    "failed to parse input as yaml into {}",
                    std::any::type_name::<T>()
                )
            })
        }
    }

    /// Parse value from a YAML `&str`
    #[inline(always)]
    pub fn parse<T: for<'a> Deserialize<'a>>(x: &str) -> crate::Result<T> {
        Yaml::<T>::parse_borrowed(x)
    }

    /// Parse value from a reader that yields YAML
    #[inline(always)]
    pub fn read<T: for<'a> Deserialize<'a>>(x: impl std::io::Read) -> crate::Result<T> {
        Yaml::<T>::parse_read(x)
    }

    /// Parse value from an async reader that yields YAML
    #[cfg(feature = "coroutine")]
    pub async fn co_read<T: for<'a> Deserialize<'a>>(
        x: impl tokio::io::AsyncRead + Unpin,
    ) -> crate::Result<T> {
        crate::co_read::<Yaml<T>>(x).await
    }

    /// Convert a YAML value into T
    pub fn from_value<T: for<'a> Deserialize<'a>>(x: Value) -> crate::Result<T> {
        use crate::Context;
        crate::check!(
            serde_yaml_ng::from_value(x),
            "failed to convert yaml value to {}",
            std::any::type_name::<T>()
        )
    }

    /// Convert a T into a YAML value
    pub fn to_value<T: ?Sized + Serialize>(x: &T) -> crate::Result<Value> {
        use crate::Context;
        crate::check!(
            serde_yaml_ng::to_value(x),
            "failed to convert {} to yaml value",
            std::any::type_name::<T>()
        )
    }

    /// Convert the value to a YAML string
    pub fn stringify<T: serde::Serialize>(x: &T) -> crate::Result<String> {
        use crate::Context;
        crate::check!(
            serde_yaml_ng::to_string(x),
            "failed to serialize {} to yaml",
            std::any::type_name::<T>()
        )
    }

    /// Serialize the value into YAML and write into the writer
    pub fn write<T: ?Sized + Serialize>(w: impl std::io::Write, x: &T) -> crate::Result<()> {
        use crate::Context;
        crate::check!(
            serde_yaml_ng::to_writer(w, x),
            "failed to write serialized {} as yaml",
            std::any::type_name::<T>()
        )
    }
}
