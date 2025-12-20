extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Fields, FieldsNamed, Ident, ItemStruct, LitStr, Meta, Type, TypeTuple, parse_macro_input,
    parse2, punctuated::Punctuated,
};

#[proc_macro_attribute]
pub fn consumer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;

    let fields = if let Fields::Named(f) = &input.fields {
        &f.named
    } else {
        panic!("Consumer macro only supports named fields");
    };

    let mut consume_methods = Vec::new();
    let mut cleaned_fields = Punctuated::new();
    let mut deserialize_switch = Vec::new();

    for field in fields {
        let name = field.ident.as_ref().unwrap();
        let mut handler_ident = None;
        let mut state_ident = None;

        for attr in &field.attrs {
            if attr.path().is_ident("topic") {
                if let Meta::List(meta) = &attr.meta {
                    if let Ok(lit) = parse2::<LitStr>(meta.tokens.clone()) {
                        handler_ident = Some((Ident::new(&lit.value(), lit.span()), &field.ty));
                    }
                }
            }

            if attr.path().is_ident("state") {
                if let Meta::List(meta) = &attr.meta {
                    if let Ok(lit) = parse2::<LitStr>(meta.tokens.clone()) {
                        state_ident = Some(Ident::new(&lit.value(), lit.span()));
                    }
                }
            }
        }

        if let Some((handler, ty)) = handler_ident {
            let consume_name = Ident::new(&format!("consume_{}", name), name.span());

            let is_unit_type = is_unit(&ty);

            let params = if is_unit_type {
                quote! { &self }
            } else {
                quote! { &self, value: #ty }
            };

            let call = match state_ident {
                Some(state) => {
                    if !is_unit_type {
                        quote! { self.#state.clone(), value }
                    } else {
                        quote! { self.#state.clone() }
                    }
                }
                None => quote! { value },
            };

            let method = {
                quote! {
                    #[inline]
                    fn #consume_name(#params) {
                        #handler(#call);
                    }
                }
            };

            let lit = LitStr::new(&name.to_string(), name.span());
            let lit_value = lit.value();

            let switch_stmt = if !is_unit_type {
                quote! {
                    self.#consume_name(postcard::from_bytes(payload_bytes)?);
                }
            } else {
                quote! { self.#consume_name(); }
            };

            deserialize_switch.push(quote! {
                #lit_value => {
                    #switch_stmt
                }
            });
            consume_methods.push(method);
        }

        if !field.attrs.iter().any(|attr| attr.path().is_ident("topic")) {
            cleaned_fields.push(field.clone());
        }
    }

    let output_struct = ItemStruct {
        attrs: input.attrs,
        vis: input.vis,
        struct_token: input.struct_token,
        ident: input.ident.clone(),
        generics: input.generics,
        fields: Fields::Named(FieldsNamed {
            brace_token: Default::default(),
            named: cleaned_fields,
        }),
        semi_token: input.semi_token,
    };

    deserialize_switch.push(quote! {
        _ => anyhow::bail!("Topic not found"),
    });

    let dispatcher = quote! {
        impl pusu::consumer::Consumer for #struct_name {
            fn dispatch(&self, topic: &str, payload_bytes: &[u8]) -> anyhow::Result<()> {
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
        }
    };

    TokenStream::from(expanded)
}

fn is_unit(ty: &Type) -> bool {
    match ty {
        Type::Tuple(TypeTuple { elems, .. }) => elems.is_empty(),
        _ => false,
    }
}
