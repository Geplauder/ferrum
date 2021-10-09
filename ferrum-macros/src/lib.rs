use proc_macro::TokenStream;
use proc_macro2::Span;

#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = syn::parse_macro_input!(item as syn::ItemFn);
    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &mut input.sig;
    let body = &input.block;

    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let mut strategy = None;

    for arg in &args {
        match arg {
            syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue {
                lit: syn::Lit::Str(lit),
                path,
                ..
            })) => match path
                .get_ident()
                .map(|i| i.to_string().to_lowercase())
                .as_deref()
            {
                Some("strategy") => strategy = Some(lit.value()),
                _ => {
                    return syn::Error::new_spanned(arg, "Unknown argument specified")
                        .to_compile_error()
                        .into()
                }
            },
            _ => {
                return syn::Error::new_spanned(arg, "Unknown argument specified")
                    .to_compile_error()
                    .into()
            }
        }
    }

    let strategy = syn::parse_str::<syn::Expr>(&format!(
        "crate::helpers::BootstrapType::{}",
        strategy.unwrap_or_else(|| "Default".to_string())
    ))
    .unwrap();

    sig.asyncness = None;

    (quote::quote_spanned! {Span::call_site()=>
        #[test]
        #(#attrs)*
        #vis #sig {
            actix_rt::System::new()
                .block_on(async {
                    let app = crate::helpers::spawn_app(#strategy).await;

                    #body

                    app.db_pool.close().await;

                    crate::helpers::teardown(&app.settings.database).await;
                })
        }
    })
    .into()
}
