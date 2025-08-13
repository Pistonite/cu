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
