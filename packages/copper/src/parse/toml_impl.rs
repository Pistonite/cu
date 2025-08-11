use serde::Deserialize;

use crate::Context as _;

use super::Parse;


pub struct Toml<T>(std::marker::PhantomData<T>);
impl<T: for <'a> Deserialize<'a>> Parse for Toml<T> {
    type Output = T;

    fn parse_borrowed(x: &str) -> crate::Result<Self::Output> {
        ::toml::from_str(x)
            .with_context(|| format!("failed to parse input as toml into {}",
            std::any::type_name::<T>()
        ))
    }
}

#[inline(always)]
pub fn parse_toml<T: for <'a> Deserialize<'a>>(x: &str) -> crate::Result<T> {
    Toml::<T>::parse_borrowed(x)
}

#[inline(always)]
pub fn parse_toml_owned<T: for <'a> Deserialize<'a>>(x: String) -> crate::Result<T> {
    Toml::<T>::parse_owned(x)
}

#[inline(always)]
pub fn read_toml<T: for <'a> Deserialize<'a>>(x: impl std::io::Read) -> crate::Result<T> {
    Toml::<T>::parse_read(x)
}

#[cfg(feature = "coroutine")]
pub async fn co_read_toml<T: for <'a> Deserialize<'a>>(x: impl tokio::io::AsyncRead + Unpin) -> crate::Result<T> {
    super::co_read::<Toml<T>>(x).await
}

pub mod toml {
    pub use ::toml::{Value, Table, Spanned};
}
pub use ::toml::toml;
