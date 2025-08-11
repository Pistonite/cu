use serde::Deserialize;

use crate::Context as _;

use super::Parse;


pub struct Yaml<T>(std::marker::PhantomData<T>);
impl<T: for <'a> Deserialize<'a>> Parse for Yaml<T> {
    type Output = T;

    fn parse_borrowed(x: &str) -> crate::Result<Self::Output> {
        serde_yaml_ng::from_str(x)
            .with_context(|| format!("failed to parse input as yaml into {}",
            std::any::type_name::<T>()
        ))
    }

    fn parse_read(x: impl std::io::Read) -> crate::Result<Self::Output> {
        serde_yaml_ng::from_reader(x)
            .with_context(|| format!("failed to parse input as yaml into {}",
            std::any::type_name::<T>()
        ))
    }
}

#[inline(always)]
pub fn parse_yaml<T: for <'a> Deserialize<'a>>(x: &str) -> crate::Result<T> {
    Yaml::<T>::parse_borrowed(x)
}

#[inline(always)]
pub fn parse_yaml_owned<T: for <'a> Deserialize<'a>>(x: String) -> crate::Result<T> {
    Yaml::<T>::parse_owned(x)
}

#[inline(always)]
pub fn read_yaml<T: for <'a> Deserialize<'a>>(x: impl std::io::Read) -> crate::Result<T> {
    Yaml::<T>::parse_read(x)
}

#[cfg(feature = "coroutine")]
pub async fn co_read_yaml<T: for <'a> Deserialize<'a>>(x: impl tokio::io::AsyncRead + Unpin) -> crate::Result<T> {
    super::co_read::<Yaml<T>>(x).await
}

/// 
pub mod yaml {
    pub use serde_yaml_ng::{Value, Mapping, Number, Sequence};
    pub use Mapping as Map;
    pub use Sequence as Vec;
}
