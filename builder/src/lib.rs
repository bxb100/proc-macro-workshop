use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Expr, Field, Meta, Type};

fn get_field_type_argument<'k>(ident: &str, field: &'k Field) -> Option<&'k Type> {
    // Option<String>
    if let Type::Path(syn::TypePath { ref path, .. }) = field.ty {
        if path.segments.len() == 1 && path.segments[0].ident == ident {
            if let syn::PathArguments::AngleBracketed(ref inner_type) = path.segments[0].arguments {
                if inner_type.args.len() != 1 {
                    return None;
                }
                if let syn::GenericArgument::Type(ref ty) = inner_type.args[0] {
                    return Some(ty);
                }
            }
        }
    }
    None
}

fn get_attr_name(field: &Field) -> Result<Option<syn::Ident>, TokenStream> {
    // #[builder(each = "env")]
    for attr in &field.attrs {
        if attr.path().is_ident("builder") {
            if let Ok(meta) = attr.parse_args::<Meta>() {
                if !meta.path().is_ident("each") {
                    return Err(syn::Error::new_spanned(
                        attr.meta.clone(),
                        "expected `builder(each = \"...\")`",
                    )
                    .into_compile_error());
                }
                if let Meta::NameValue(syn::MetaNameValue {
                    value:
                        Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit_str),
                            ..
                        }),
                    ..
                }) = meta
                {
                    return Ok(Some(format_ident!("{}", lit_str.value())));
                }
            }
        }
    }
    Ok(None)
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let DeriveInput { ident, data, .. } = input;

    let fields = if let Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = data
    {
        named
    } else {
        unimplemented!()
    };

    // TODO: pass 08, does there have a better solution?
    for f in fields {
        if let Err(e) = get_attr_name(f) {
            return e.into();
        }
    }

    let builder_struct_fields = fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;
        if get_field_type_argument("Option", field).is_some()
            || get_attr_name(field).unwrap().is_some()
        {
            quote! {
                #ident: #ty
            }
        } else {
            quote! {
                #ident: ::std::option::Option<#ty>
            }
        }
    });
    let builder_new_fields = fields.iter().map(|field| {
        let ident = &field.ident;
        if get_attr_name(field).unwrap().is_some() {
            quote! {
                #ident: ::std::vec![]
            }
        } else {
            quote! {
                #ident: ::std::option::Option::None
            }
        }
    });
    let builder_methods = fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;
        // TIPS: No need check error here, because we put in the first, don't meddle this generation
        if let Some(each) = get_attr_name(field).unwrap() {
            if let Some(inner_type) = get_field_type_argument("Vec", field) {
                return quote! {
                    pub fn #each(&mut self, #each: #inner_type) -> &mut Self {
                        self.#ident.push(#each);
                        self
                    }
                };
            }
        }

        if let Some(inner_type) = get_field_type_argument("Option", field) {
            quote! {
                pub fn #ident(&mut self, #ident: #inner_type) -> &mut Self {
                    self.#ident = Some(#ident);
                    self
                }
            }
        } else {
            quote! {
              pub fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = Some(#ident);
                self
              }
            }
        }
    });
    let build_fields = fields.iter().map(|field| {
        let ident = &field.ident;
        if get_field_type_argument("Option", field).is_some()
            || get_attr_name(field).unwrap().is_some()
        {
            quote! {
                #ident: self.#ident.clone()
            }
        } else {
            quote! {
                #ident: self.#ident.clone().ok_or("filed was not set")?
            }
        }
    });

    let builder_name = format_ident!("{}Builder", ident);

    let builder = quote! {

        pub struct #builder_name {
            #( #builder_struct_fields ),*
        }

        impl #builder_name {
            #( #builder_methods )*

            fn build(self: &mut Self) -> ::std::result::Result<#ident, ::std::boxed::Box<dyn std::error::Error>> {
                ::std::result::Result::Ok(
                    #ident {
                        #( #build_fields ),*
                    }
                )
            }
        }

        impl #ident {
            fn builder() -> #builder_name {
                #builder_name {
                    #( #builder_new_fields ),*
                }
            }
        }
    };

    builder.into()
}
