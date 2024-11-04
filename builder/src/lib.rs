use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::*;

#[proc_macro_derive(Builder, attributes(builder))]
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
        match field_type(field) {
            Ok(field_type) => {
                builder_defaults.push(builder_default(field));
                builder_field_definitions.push(builder_field_definition(field, &field_type));
                builder_setters.push(builder_setter(field, &vis, &field_type));
                build_attrs.push(build_attr(field, &field_type));
            }
            Err(err) => return err.to_compile_error().into(),
        }
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

fn builder_field_definition(
    Field { ident, ty, .. }: &Field,
    field_type: &FieldType,
) -> TokenStream2 {
    let ty = match field_type {
        &FieldType::Optional(ty) => ty,
        &FieldType::Required(ty) => ty,
        &FieldType::Repeated(_, _) => ty,
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
    match field_type {
        FieldType::Optional(ty) | FieldType::Required(ty) => quote! {
            #vis fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = Some(#ident);
                self
            }
        },
        FieldType::Repeated(ty, each) => quote! {
            #vis fn #each(&mut self, #each: #ty) -> &mut Self {
                self.#ident.get_or_insert_with(Vec::new).push(#each);
                self
            }
        },
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
        FieldType::Repeated(_, _) => quote! {
            #ident: self.#ident.clone().unwrap_or_default(),
        },
    }
}

#[derive(Debug)]
enum FieldType<'a> {
    Optional(&'a Type),
    Required(&'a Type),
    Repeated(&'a Type, Ident),
}

fn field_type<'a>(Field { ty, attrs, .. }: &'a Field) -> Result<FieldType<'a>> {
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
                    return Ok(FieldType::Optional(ty));
                }
            } else if ident == "Vec" {
                if let Some(each) = each_attr(&attrs) {
                    if let Some(GenericArgument::Type(ty)) = args.first() {
                        return Ok(FieldType::Repeated(ty, each?));
                    }
                }
            }
        }
    }
    return Ok(FieldType::Required(ty));
}

fn each_attr(attrs: &Vec<Attribute>) -> Option<Result<Ident>> {
    let mut each: Option<Result<Ident>> = None;

    for attr in attrs {
        if !attr.path().is_ident("builder") {
            continue;
        }
        if let Meta::List(meta_list) = &attr.meta {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("each") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    each = Some(Ok(format_ident!("{}", s.value())));
                } else {
                    each = Some(Err(Error::new_spanned(
                        meta_list,
                        "expected `builder(each = \"...\")`",
                    )));
                }
                Ok(())
            });
        }
    }

    each
}
