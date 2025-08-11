//! Batteries-included proc-macro utils
//!
//! This re-exports the common libraries - `syn`, `quote`, and `proc-macro2`,
//! that you need to work with proc-macros, as well as utilities
//! to interop between them.
//!
//! # Re-exports
//! Most of `quote`, `proc_macro`, and `proc_macro2` are re-exported,
//! and they can be used with the `pm::` prefix. Types that have the same
//! name in `proc_macro` and `proc_macro2` are suffixed with `2` in
//! the `proc_macro2` export.
//!
//! Types from `syn` can be used as with `syn::` prefix, by adding
//! the prelude import `use pm::pre::*`.

mod util;
pub use util::*;

// lib re-exports
pub mod lib {
    pub use syn;
    pub use quote;
    pub use proc_macro2;
}

// type re-exports
pub use quote::{format_ident, quote, quote_spanned, ToTokens};
pub use syn::{Result, Error};
pub use proc_macro2::{
    Group as Group2,
    Ident as Ident2,
    LexError as LexError2,
    Literal as Literal2,
    Span as Span2,
    TokenStream as TokenStream2,
    Punct as Punct2,
    Delimiter as Delimiter2,
    Spacing as Spacing2,
    TokenTree as TokenTree2,
};

#[cfg(feature = "proc-macro")]
extern crate proc_macro;
#[cfg(feature = "proc-macro")]
pub use proc_macro::{
    Group,
    Ident,
    LexError,
    Literal, Punct, Span, TokenStream,
    Delimiter, Spacing,
    TokenTree,
};

// prelude traits
pub mod pre {
    pub use syn;
    pub use crate::lib::quote::ToTokens as _;
    pub use crate::TokenStream;
    pub use crate::TokenStream2;
    pub use crate::Span;
    pub use crate::Span2;
}
