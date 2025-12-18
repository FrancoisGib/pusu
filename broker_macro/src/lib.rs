extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input, spanned::Spanned};

#[proc_macro_attribute]
pub fn broker(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = &input.ident;

    let fields = if let syn::Data::Struct(data) = &input.data {
        if let syn::Fields::Named(fields_named) = &data.fields {
            fields_named.named.iter().collect::<Vec<_>>()
        } else {
            panic!("Broker can only be derived for structs with named fields");
        }
    } else {
        panic!("Broker can only be derived for structs");
    };

    let mut fields_declaration = vec![];
    let mut init_fields = vec![];
    let mut publish_methods = vec![];
    let mut consume_methods = vec![];

    for field in fields.iter() {
        let name = &field.ident;
        let ty = &field.ty;

        init_fields.push(quote! {
            #name: pusu::Topic::<#ty>::new(stringify!(#name))
        });

        let publish_name =
            syn::Ident::new(&format!("publish_{}", name.as_ref().unwrap()), name.span());
        let consume_name =
            syn::Ident::new(&format!("consume_{}", name.as_ref().unwrap()), name.span());

        publish_methods.push(quote! {
            pub fn #publish_name(&mut self, payload: #ty) {
                self.#name.publish(payload);
            }
        });

        consume_methods.push(quote! {
            pub fn #consume_name(&mut self) -> Option<pusu::Message<#ty>> {
                self.#name.consume()
            }
        });

        fields_declaration.push(quote! {
            #name: pusu::Topic<#ty>
        });
    }

    let expanded = quote! {
        struct #struct_name {
            #(#fields_declaration),*
        }

        impl #struct_name {
            pub fn new() -> Self {
                Self {
                    #(#init_fields),*
                }
            }

            #(#publish_methods)*
            #(#consume_methods)*
        }
    };

    TokenStream::from(expanded)
}
