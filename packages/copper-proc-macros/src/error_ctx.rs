use pm::pre::*;

pub fn expand(attr: TokenStream, input: TokenStream) -> pm::Result<TokenStream2> {
    let attrs = parse_attributes(attr)?;
    let item: syn::ItemFn = syn::parse(input)?;

    let item_attrs = &item.attrs;
    let item_block = &item.block;
    let sig = &item.sig;
    let is_async = item.sig.asyncness.is_some();
    let retty = match &item.sig.output {
        syn::ReturnType::Default => pm::quote! {()},
        syn::ReturnType::Type(_, ty) => pm::quote! {#ty},
    };
    let args = attrs.format_args;

    let block = match (attrs.is_pre, is_async) {
        // for non-async, we need to use a closure
        // to prevent `?` operator from returning directly.
        // for async, we can use an async block
        (true, false) => {
            pm::quote! {
                use cu::Context as _;
                let __error_msg = format!(#args);
                let __result: #retty = (move|| -> #retty #item_block)();
                __result.context(__error_msg)
            }
        }
        (true, true) => {
            pm::quote! {
                use cu::Context as _;
                let __error_msg = format!(#args);
                let __result: #retty = async move #item_block.await;
                __result.context(__error_msg)
            }
        }
        (false, false) => {
            pm::quote! {
                use cu::Context as _;
                let __result: #retty = (move|| -> #retty #item_block)();
                __result.with_context(|| format!(#args))
            }
        }
        (false, true) => {
            pm::quote! {
                use cu::Context as _;
                let __result: #retty = async move #item_block.await;
                __result.with_context(|| format!(#args))
            }
        }
    };

    let expanded = pm::quote! {
        #(#item_attrs)* #sig { #block }
    };

    Ok(expanded)
}

fn parse_attributes(attr: TokenStream) -> pm::Result<ParsedAttributes> {
    let Ok(attrs) = pm::parse_punctuated::<syn::Meta, syn::Token![,]>(attr.clone()) else {
        // if the input is not a list of meta, assuming using shorthand
        // input is format args
        return Ok(ParsedAttributes {
            format_args: attr.into(),
            ..Default::default()
        });
    };

    let mut out = ParsedAttributes::default();
    for attr in attrs {
        match attr {
            syn::Meta::Path(attr) => {
                if attr.is_ident("pre") {
                    out.is_pre = true;
                    continue;
                } else if attr.is_ident("format") {
                    pm::bail!(
                        attr,
                        "`format` attribute should contain the format args, i.e. #[cu::error_ctx(format(...))], or use the shorthand #[cu::error_ctx(...)]"
                    );
                }
                pm::bail!(attr, "unknown attribute")
            }
            syn::Meta::List(attr) => {
                if attr.path.is_ident("pre") {
                    pm::bail!(
                        attr,
                        "`pre` attribute should not have a value, i.e. #[cu::error_ctx(pre, ...)]"
                    );
                }
                if attr.path.is_ident("format") {
                    out.format_args = attr.tokens;
                    continue;
                }
                pm::bail!(attr, "unknown attribute")
            }
            syn::Meta::NameValue(attr) => {
                if attr.path.is_ident("pre") {
                    pm::bail!(
                        attr,
                        "`pre` attribute should not have a value, i.e. #[cu::error_ctx(pre, ...)]"
                    );
                }
                if attr.path.is_ident("format") {
                    pm::bail!(
                        attr,
                        "`format` attribute should be parenthesized, i.e. #[cu::error_ctx(format(...))], or use the shorthand #[cu::error_ctx(...)]"
                    );
                }
                pm::bail!(attr, "unknown attribute")
            }
        }
    }

    Ok(out)
}

#[derive(Default)]
struct ParsedAttributes {
    /// if the error string should be formatted before invoking
    /// the function. this is needed if some non-Copy values
    /// are moved into the function
    is_pre: bool,

    /// The format expression
    format_args: TokenStream2,
}
