use pm::pre::*;

/// See documentation for [`cu::cli`](../pistonite-cu/cli/index.html) module
#[proc_macro_attribute]
pub fn cli(attr: TokenStream, input: TokenStream) -> TokenStream {
    pm::flatten(cli::expand(attr, input))
}
mod cli;

/// Derive the [`cu::Parse`](../pistonite-cu/trait.Parse.html) trait
#[proc_macro_derive(Parse)]
pub fn derive_parse(input: TokenStream) -> TokenStream {
    pm::flatten(derive_parse::expand(input))
}
mod derive_parse;

/// Attribute macro for wrapping a function with an error context
///
/// See the [tests](https://github.com/Pistonite/cu/blob/main/packages/copper/tests/error_ctx.rs)
/// for examples
#[proc_macro_attribute]
pub fn error_ctx(attr: TokenStream, input: TokenStream) -> TokenStream {
    pm::flatten(error_ctx::expand(attr, input))
}
mod error_ctx;
