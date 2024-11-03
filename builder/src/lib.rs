use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let DeriveInput {
        ident,
        vis,
        generics,
        ..
    } = input;

    let builder_ident = format_ident!("{}Builder", ident);

    let expanded = quote! {
        impl #generics #ident #generics {
            #vis fn builder() -> #builder_ident #generics {
                #builder_ident {
                    executable: None,
                    args: None,
                    env: None,
                    current_dir: None,
                }
            }
        }

        #vis struct #builder_ident #generics {
            executable: Option<String>,
            args: Option<Vec<String>>,
            env: Option<Vec<String>>,
            current_dir: Option<String>,
        }
    };

    TokenStream::from(expanded)
}
