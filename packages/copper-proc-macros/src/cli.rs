use pm::pre::*;

pub fn expand(attr: TokenStream, input: pm::TokenStream) -> pm::Result<TokenStream2> {
    let flags_ident = {
        if attr.is_empty() {
            None
        } else {
            let attr: syn::MetaNameValue = syn::parse(attr)?;
            if !attr.path.is_ident("flags") {
                pm::bail!(attr, "unknown attribute");
            }
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = attr.value
            else {
                pm::bail!(attr.value, "expecting string literal");
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
            None => pm::quote! { cu::cli::co_run(#generated_main_name) },
            Some(flags) => pm::quote! { cu::cli::__co_run(#generated_main_name, |x| &x.#flags) },
        }
    } else {
        match flags_ident {
            None => pm::quote! { cu::cli::run(#generated_main_name) },
            Some(flags) => pm::quote! { cu::cli::__run(#generated_main_name, |x| &x.#flags) },
        }
    };

    item.sig.ident = generated_main_name;

    let expanded = pm::quote! {
        fn main() -> std::process::ExitCode {
            unsafe { #main_impl }
        }
        #item
    };

    Ok(expanded)
}
