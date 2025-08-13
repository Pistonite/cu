/// Bail with a `syn::Error`. Use from a function that returns `syn::Result`.
#[macro_export]
macro_rules! bail {
    ($tokens:expr, $msg:expr) => {
        return Err($crate::lib::syn::Error::new_spanned($tokens, $msg))
    };
    ($tokens:expr, $($tt:tt)*) => {
        return Err($crate::lib::syn::Error::new_spanned($tokens, format!($($tt)*)))
    };
}

/// Flatten an expansion result or error into token stream.
///
/// This is used at the top-level to convert result from expansion implementation
/// to the return value expected by Rust
#[cfg(feature = "proc-macro")]
pub fn flatten<T: crate::ToTokens>(result: syn::Result<T>) -> crate::TokenStream {
    match result {
        Ok(x) => x.into_token_stream().into(),
        Err(e) => e.into_compile_error().into(),
    }
}
