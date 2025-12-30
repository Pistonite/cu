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

/// Convenience wrapper for parsing punctuated syntax
#[cfg(feature = "proc-macro")]
pub fn parse_punctuated<T: syn::parse::Parse, P: syn::parse::Parse>(
    input: crate::TokenStream,
) -> syn::Result<syn::punctuated::Punctuated<T, P>> {
    let pun: Punctuated<T, P> = syn::parse(input)?;
    Ok(pun.0)
}

/// Convenience wrapper for parsing punctuated syntax
pub fn parse_punctuated2<T: syn::parse::Parse, P: syn::parse::Parse>(
    input: crate::TokenStream2,
) -> syn::Result<syn::punctuated::Punctuated<T, P>> {
    let pun: Punctuated<T, P> = syn::parse2(input)?;
    Ok(pun.0)
}

struct Punctuated<T, P>(syn::punctuated::Punctuated<T, P>);
impl<T: syn::parse::Parse, P: syn::parse::Parse> syn::parse::Parse for Punctuated<T, P> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let parts = syn::punctuated::Punctuated::<T, P>::parse_terminated(input)?;
        Ok(Self(parts))
    }
}
