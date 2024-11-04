use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
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

    let mut builder_defaults = Vec::with_capacity(fields.len());
    let mut builder_field_definitions = Vec::with_capacity(fields.len());
    let mut builder_setters = Vec::with_capacity(fields.len());
    let mut build_attrs = Vec::with_capacity(fields.len());

    for field in &fields {
        let ft = field_type(field);
        builder_defaults.push(builder_default(field));
        builder_field_definitions.push(builder_field_definition(field, &ft));
        builder_setters.push(builder_setter(field, &vis, &ft));
        build_attrs.push(build_attr(field, &ft));
    }

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

fn builder_default(Field { ident, .. }: &Field) -> TokenStream2 {
    quote! {
        #ident: None,
    }
}

fn builder_field_definition(Field { ident, .. }: &Field, field_type: &FieldType) -> TokenStream2 {
    let ty = match field_type {
        &FieldType::Optional(ty) => ty,
        &FieldType::Required(ty) => ty,
    };
    quote! {
        #ident: Option<#ty>,
    }
}

fn builder_setter(
    Field { ident, .. }: &Field,
    vis: &Visibility,
    field_type: &FieldType,
) -> TokenStream2 {
    let ty = match field_type {
        &FieldType::Optional(ty) => ty,
        &FieldType::Required(ty) => ty,
    };
    quote! {
        #vis fn #ident(&mut self, #ident: #ty) -> &mut Self {
            self.#ident = Some(#ident);
            self
        }
    }
}

fn build_attr(Field { ident, .. }: &Field, field_type: &FieldType) -> TokenStream2 {
    match field_type {
        FieldType::Optional(_) => quote! {
            #ident: self.#ident.clone(),
        },
        FieldType::Required(_) => {
            let ident = ident.clone().unwrap();
            let message = format!("field {} isn't set", ident);
            quote! {
                #ident: self.#ident
                    .clone()
                    .ok_or_else(|| -> Box<dyn std::error::Error> { #message.into() })?,
            }
        }
    }
}

#[derive(Debug)]
enum FieldType<'a> {
    Optional(&'a Type),
    Required(&'a Type),
}

fn field_type(Field { ty, .. }: &Field) -> FieldType {
    if let Type::Path(TypePath {
        path: Path { segments, .. },
        ..
    }) = ty
    {
        if let Some(PathSegment {
            ident,
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }),
        }) = segments.first()
        {
            if ident == "Option" {
                if let Some(GenericArgument::Type(ty)) = args.first() {
                    return FieldType::Optional(ty);
                }
            }
        }
    }
    return FieldType::Required(ty);
}
