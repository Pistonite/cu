use pm::pre::*;

pub fn expand(attr: TokenStream, input: TokenStream) -> pm::Result<TokenStream2> {
    let attrs = parse_attributes(attr)?;

    let mut item: syn::ItemFn = syn::parse(input)?;
    let is_async = item.sig.asyncness.is_some();

    let generated_main_name = {
        let ident = &item.sig.ident;
        let ident_str = ident.to_string();
        let gen_ident = format!("__cu{}{ident_str}", ident_str.len());
        syn::Ident::new(&gen_ident, ident.span())
    };

    let fn_flag_impl = match attrs.flags_ident {
        Some(flags) => pm::quote! { |x| &x.#flags },
        None => pm::quote! { |x| x.as_ref() },
    };

    let fn_preproc_impl = match attrs.preprocess_fn {
        Some(value) => pm::quote! { { #value } },
        None => pm::quote! { (|_| {}) },
    };

    let main_impl = if is_async {
        pm::quote! {
            cu::cli::__co_run(
                #fn_preproc_impl,
                #generated_main_name,
                #fn_flag_impl
            )
        }
    } else {
        pm::quote! {
            cu::cli::__run(
                #fn_preproc_impl,
                #generated_main_name,
                #fn_flag_impl
            )
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

fn parse_attributes(attr: TokenStream) -> pm::Result<ParsedAttributes> {
    let attrs = pm::parse_punctuated::<syn::MetaNameValue, syn::Token![,]>(attr)?;
    let mut out = ParsedAttributes::default();

    for attr in attrs {
        if attr.path.is_ident("flags") {
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = attr.value
            else {
                pm::bail!(attr.value, "expecting string literal");
            };
            let ident: syn::Ident = s.parse()?;
            out.flags_ident = Some(ident);
            continue;
        }
        if attr.path.is_ident("preprocess") {
            out.preprocess_fn = Some(attr.value);
            continue;
        }
        pm::bail!(attr, "unknown attribute");
    }
    Ok(out)
}
#[derive(Default)]
struct ParsedAttributes {
    flags_ident: Option<syn::Ident>,
    preprocess_fn: Option<syn::Expr>,
}
