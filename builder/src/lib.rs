use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::*;

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let DeriveInput {
        ident,
        vis,
        generics,
        data,
        ..
    } = input;

    let builder_ident = format_ident!("{}Builder", ident);

    let fields = match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => named,
        _ => {
            return quote! {
                compile_error!("Builder derive only works on structs with named fields");
            }
            .into();
        }
    };

    let builder_defaults = fields.iter().map(|field| {
        let field_name = &field.ident;
        quote! {
            #field_name: None,
        }
    });

    let builder_field_definitions = fields.iter().map(|field| {
        let field_name = &field.ident;
        let ty = &field.ty;
        quote! {
            #field_name: Option<#ty>,
        }
    });

    let builder_setters = fields.iter().map(|field| {
        let field_name = &field.ident;
        let ty = &field.ty;
        quote! {
            #vis fn #field_name(&mut self, #field_name: #ty) -> &mut Self {
                self.#field_name = Some(#field_name);
                self
            }
        }
    });

    let build_attrs = fields.iter().map(|field| {
        let field_ident = field.ident.clone().unwrap();
        let message = format!("field {} isn't set", field_ident);
        quote! {
            #field_ident: self.#field_ident
                .clone()
                .ok_or_else(|| -> Box<dyn std::error::Error> { #message.into() })?,
        }
    });

    let expanded = quote! {
        impl #generics #ident #generics {
            #vis fn builder() -> #builder_ident #generics {
                #builder_ident {
                    #(#builder_defaults)*
                }
            }
        }

        #vis struct #builder_ident #generics {
            #(#builder_field_definitions)*
        }

        impl #generics #builder_ident #generics {
            #(#builder_setters)*
            #vis fn build(&mut self) -> Result<#ident #generics, Box<dyn std::error::Error>> {
                Ok(#ident {
                    #(#build_attrs)*
                })
            }
        }
    };

    TokenStream::from(expanded)
}
