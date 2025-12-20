extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_attribute]
pub fn broker(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = &input.ident;

    let fields = if let Data::Struct(data) = &input.data {
        if let Fields::Named(fields_named) = &data.fields {
            fields_named.named.iter().collect::<Vec<_>>()
        } else {
            panic!("Broker can only be derived for structs with named fields");
        }
    } else {
        panic!("Broker can only be derived for structs");
    };

    let mut fields_declaration = vec![];
    let mut init_fields = vec![];

    for field in fields.iter() {
        let name = &field.ident;
        let ty = &field.ty;

        init_fields.push(quote! {
            #name: pusu::broker::Topic::<#ty>::new(stringify!(#name))
        });

        fields_declaration.push(quote! {
            #name: pusu::broker::Topic<#ty>
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
        }
    };

    TokenStream::from(expanded)
}
