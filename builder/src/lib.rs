use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    println!("{:#?}", input);

    let DeriveInput { ident, data, .. } = input;

    let builder_name = format_ident!("{}Builder", ident);

    let fields = if let Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = data
    {
        named
    } else {
        unimplemented!()
    };
    
    // build a `Indent + Builder` struct
    let builder_struct = {
        let fields = fields.iter().map(|field| {
            let ident = &field.ident;
            let ty = &field.ty;
            quote! {
                #ident: Option<#ty>,
            }
        });
        quote! {
            pub struct #builder_name {
                #(#fields)*
            }
        }
    };
    let builder_struct_impl = {
        let functions = fields.iter().map(|field| {
            let ident = &field.ident;
            let ty = &field.ty;
            quote! {
              pub fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = Some(#ident);
                self
              }
            }
        });

        let builder_fields = fields.iter().map(|field| {
            let ident = &field.ident;
            quote! {
                #ident: self.#ident.clone().ok_or("filed was not set")?,
            }
        });
        quote! {
            impl #builder_name {
                #(#functions)*
                
                fn build(&mut self) -> Result<#ident, Box<dyn std::error::Error>> {
                    Ok(#ident {
                        #(#builder_fields)*
                    })
                }
            }
        }
    };
    // build a impl with function `builder` that returns a `Indent + Builder` struct
    let builder = {
        let fields = fields.iter().map(|field| {
            let ident = &field.ident;
            quote! {
                #ident: None,
            }
        });
        quote! {
            impl #ident {
                pub fn builder() -> #builder_name {
                    #builder_name {
                        #(#fields)*
                    }
                }
            }
        }
    };
    let expand = quote! {
        #builder_struct
        #builder_struct_impl
        #builder
    };

    proc_macro::TokenStream::from(expand)
}
