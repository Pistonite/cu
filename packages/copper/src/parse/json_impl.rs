use serde::Deserialize;

use crate::Context as _;

use super::Parse;


pub struct Json<T>(std::marker::PhantomData<T>);
impl<T: for <'a> Deserialize<'a>> Parse for Json<T> {
    type Output = T;

    fn parse_borrowed(x: &str) -> crate::Result<Self::Output> {
        serde_json::from_str(x)
            .with_context(|| format!("failed to parse input as json into {}",
            std::any::type_name::<T>()
        ))
    }

    fn parse_read(x: impl std::io::Read) -> crate::Result<Self::Output> {
        serde_json::from_reader(x)
            .with_context(|| format!("failed to parse input as json into {}",
            std::any::type_name::<T>()
        ))
    }
}

#[inline(always)]
pub fn parse_json<T: for <'a> Deserialize<'a>>(x: &str) -> crate::Result<T> {
    Json::<T>::parse_borrowed(x)
}

#[inline(always)]
pub fn parse_json_owned<T: for <'a> Deserialize<'a>>(x: String) -> crate::Result<T> {
    Json::<T>::parse_owned(x)
}

#[inline(always)]
pub fn read_json<T: for <'a> Deserialize<'a>>(x: impl std::io::Read) -> crate::Result<T> {
    Json::<T>::parse_read(x)
}

#[cfg(feature = "coroutine")]
pub async fn co_read_json<T: for <'a> Deserialize<'a>>(x: impl tokio::io::AsyncRead + Unpin) -> crate::Result<T> {
    super::co_read::<Json<T>>(x).await
}

pub mod json {
    pub use serde_json::{Value, Map, Number, from_slice, to_value, to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer, to_writer_pretty};
}
#[cfg_attr(any(docsrs, feature = "nightly"), doc(cfg(feature = "json")))]
pub use serde_json::json;
