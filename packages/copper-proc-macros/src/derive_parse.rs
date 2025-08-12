use pm::pre::*;

pub fn expand(input: pm::TokenStream) -> pm::Result<TokenStream2> {
    let input = syn::parse::<syn::DeriveInput>(input)?;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let ident = &input.ident;
    let error_message = format!("failed to parse {ident}");

    let expanded = pm::quote! {
        #[automatically_derived]
        impl #impl_generics cu::Parse for #ident #type_generics #where_clause {
            type Output = Self;
            fn parse_borrowed(x: &str) -> cu::Result<Self> {
                use cu::Context as _;
                <Self as ::std::str::FromStr>::from_str(x).context(#error_message)
            }
        }
    };

    Ok(expanded)
}
