extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn consumer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemStruct);
    let struct_name = &input.ident;

    let fields = if let syn::Fields::Named(f) = &input.fields {
        &f.named
    } else {
        panic!("Consumer macro only supports named fields");
    };

    let mut consume_methods = Vec::new();
    let mut deserialize_methods = Vec::new();
    let mut cleaned_fields = syn::punctuated::Punctuated::new();
    let mut deserialize_switch = Vec::new();

    for field in fields {
        let name = field.ident.as_ref().unwrap();
        let mut handler_ident = None;
        let mut state_ident = None;

        for attr in &field.attrs {
            if attr.path().is_ident("topic") {
                if let syn::Meta::List(meta) = &attr.meta {
                    if let Ok(lit) = syn::parse2::<syn::LitStr>(meta.tokens.clone()) {
                        handler_ident =
                            Some((syn::Ident::new(&lit.value(), lit.span()), field.ty.clone()));
                    }
                }
            }

            if attr.path().is_ident("state") {
                if let syn::Meta::List(meta) = &attr.meta {
                    if let Ok(lit) = syn::parse2::<syn::LitStr>(meta.tokens.clone()) {
                        state_ident = Some(syn::Ident::new(&lit.value(), lit.span()));
                    }
                }
            }
        }

        if let Some((handler, ty)) = handler_ident {
            let consume_name = syn::Ident::new(&format!("consume_{}", name), name.span());
            let method = {
                match state_ident {
                    Some(state) => {
                        quote! {
                            #[inline]
                            fn #consume_name(&self, value: #ty) {
                                #handler(self.#state.clone(), value);
                            }
                        }
                    }
                    None => quote! {
                        #[inline]
                        fn #consume_name(&self, value: #ty) {
                            println!("ic");
                            #handler(value);
                        }
                    },
                }
            };

            let deserializer_name = syn::Ident::new(&format!("deserialize_{}", name), name.span());
            deserialize_methods.push(quote! {
                #[inline]
                fn #deserializer_name<'de, D>(&self, de: D) -> Result<#ty, String>
                where
                    D: serde::Deserializer<'de>,
                    #ty: serde::Deserialize<'de>,
                {
                    #ty::deserialize(de).map_err(|err| err.to_string())
                }
            });

            let lit = syn::LitStr::new(&format!("{}", name), name.span());
            let lit_value = lit.value();

            deserialize_switch.push(quote! {
                #lit_value => {
                    let value = self.#deserializer_name(&mut de)?;
                    self.#consume_name(value);
                },
            });

            consume_methods.push(method);
        }

        if !field.attrs.iter().any(|attr| attr.path().is_ident("topic")) {
            cleaned_fields.push(field.clone());
        }
    }

    let output_struct = syn::ItemStruct {
        attrs: input.attrs,
        vis: input.vis,
        struct_token: input.struct_token,
        ident: input.ident.clone(),
        generics: input.generics,
        fields: syn::Fields::Named(syn::FieldsNamed {
            brace_token: Default::default(),
            named: cleaned_fields,
        }),
        semi_token: input.semi_token,
    };

    deserialize_switch.push(quote! {
        _ => return Err("Topic not found".to_string()),
    });

    let dispatcher = quote! {
        impl pusu::consumer::Consumer for #struct_name {
            fn dispatch(&self, topic: &str, stream: &mut std::io::BufReader<std::net::TcpStream>) -> Result<(), String> {
                let mut de = serde_json::Deserializer::from_reader(stream);
                match topic {
                    #(#deserialize_switch)*
                }
                Ok(())
            }
        }
    };

    let expanded = quote! {
        #output_struct

        #dispatcher

        impl #struct_name {
            #(#consume_methods)*

            #(#deserialize_methods)*
        }
    };

    TokenStream::from(expanded)
}
