use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/// See documentation for [`cu::cli`] module
#[proc_macro_attribute]
pub fn cli(attr: TokenStream, input: TokenStream) -> TokenStream {
    unwrap_syn(expand_cli(attr, input))
}

fn expand_cli(attr: TokenStream, input: TokenStream) -> syn::Result<TokenStream2> {
    let flags_ident = {
        if attr.is_empty() {
            None
        } else {
            let attr: syn::MetaNameValue = syn::parse(attr)?;
            if !attr.path.is_ident("flags") {
                return Err(syn::Error::new_spanned(attr, "unknown attribute"));
            }
            let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = attr.value else {
                return Err(syn::Error::new_spanned(attr.value, "expecting string literal"));
            };
            let ident: syn::Ident = s.parse()?;
            Some(ident)

        }
    };
    let mut item: syn::ItemFn = syn::parse(input)?;
    let is_async = item.sig.asyncness.is_some();

    let generated_main_name = {
        let ident = &item.sig.ident;
        let ident_str = ident.to_string();
        let gen_ident = format!("__cu{}{ident_str}", ident_str.len());
        syn::Ident::new(&gen_ident, ident.span())
    };

    let main_impl = if is_async {
        match flags_ident {
            None => {
                quote! {
                    unsafe { cu::cli::co_run(#generated_main_name) }
                }
            }
            Some(flags) => {
                quote! {
                    unsafe { cu::cli::__co_run(#generated_main_name, |x| &x.#flags) }
                }
            }
        }
    } else {
        match flags_ident {
            None => {
                quote! {
                    unsafe { cu::cli::run(#generated_main_name) }
                }
            }
            Some(flags) => {
                quote! {
                    unsafe { cu::cli::__run(#generated_main_name, |x| &x.#flags) }
                }
            }
        }
    };

    item.sig.ident = generated_main_name;

    let expanded = quote! {
        fn main() -> std::process::ExitCode {
            #main_impl
        }
        #item
    };

    Ok(expanded)
}

fn unwrap_syn<T: Into<TokenStream2>>(result: syn::Result<T>) -> TokenStream {
    match result {
        Ok(x) => <T as Into<TokenStream2>>::into(x).into(),
        Err(e) => e.into_compile_error().into()
    }
}
