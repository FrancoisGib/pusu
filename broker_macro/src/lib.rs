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
                            fn #consume_name(&self, value: #ty) {
                                #handler(#state, value);
                            }
                        }
                    }
                    None => quote! {
                        fn #consume_name(&self, value: #ty) {
                            #handler(value);
                        }
                    },
                }
            };

            let deserializer_name = syn::Ident::new(&format!("deserialize_{}", name), name.span());
            deserialize_methods.push(quote! {
                fn #deserializer_name(&self, stream: &mut std::io::BufReader<std::net::TcpStream>) -> Result<#ty, String> {
                    let mut de = serde_json::Deserializer::from_reader(stream);
                    let value = #ty::deserialize(&mut de).map_err(|err| err.to_string())?;
                    Ok(value)
                }
            });

            let lit = syn::LitStr::new(&format!("{}", name), name.span());
            let lit_value = lit.value();

            deserialize_switch.push(quote! {
                #lit_value => {
                    let value = self.#deserializer_name(stream)?;
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
        fn dispatch(&self, topic: &str, stream: &mut std::io::BufReader<std::net::TcpStream>) -> Result<(), String> {
            match topic {
                #(#deserialize_switch)*
            }
            Ok(())
        }
    };

    let start_function = quote! {
        pub fn start(&self, port: u16) -> Result<(), String> {
            let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).map_err(|err| err.to_string())?;

            loop {
                if let Ok((stream, _addr)) = listener.accept() {
                    let mut buf_reader = std::io::BufReader::new(stream);
                    let mut buf = String::with_capacity(32);

                    if std::io::BufRead::read_line(&mut buf_reader, &mut buf).is_err() {
                        continue;
                    }

                    if !buf.starts_with("topic: ") {
                        continue;
                    }

                    let topic = buf[7..].trim_end();

                    let _ = self.dispatch(topic, &mut buf_reader).inspect_err(|err| println!("{}", err));
                }
            }
        }
    };

    let expanded = quote! {
        #output_struct

        impl #struct_name {
            #(#consume_methods)*

            #(#deserialize_methods)*

            #dispatcher

            #start_function
        }
    };

    TokenStream::from(expanded)
}
