extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Fields, FieldsNamed, Ident, ItemStruct, LitStr, Meta, Type, TypeTuple, Variant, Visibility,
    parse_macro_input, parse_quote, parse2,
    punctuated::Punctuated,
    token::{Comma, Enum},
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
    let mut enum_variants = Punctuated::<Variant, Comma>::new();

    let enum_name = format!("{}Topic", struct_name);
    let enum_ident = Ident::new(&enum_name, Span::call_site());

    for field in fields {
        let name = field.ident.as_ref().unwrap();
        let mut handler_ident = None;
        let mut state_ident = None;

        for attr in &field.attrs {
            if attr.path().is_ident("topic")
                && let Meta::List(meta) = &attr.meta
                && let Ok(lit) = parse2::<LitStr>(meta.tokens.clone())
            {
                handler_ident = Some((Ident::new(&lit.value(), lit.span()), &field.ty));
            }

            if attr.path().is_ident("state")
                && let Meta::List(meta) = &attr.meta
                && let Ok(lit) = parse2::<LitStr>(meta.tokens.clone())
            {
                state_ident = Some(Ident::new(&lit.value(), lit.span()));
            }
        }

        if let Some((handler, ty)) = handler_ident {
            let consume_name = Ident::new(&format!("consume_{}", name), name.span());

            let is_unit_type = is_unit(ty);

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

            let topic_lit = LitStr::new(&name.to_string(), name.span());
            let topic_str = topic_lit.value();

            let variant_ident = Ident::new(&topic_str.to_case(Case::Pascal), Span::call_site());
            let enum_variant = Variant {
                attrs: Vec::new(),
                ident: variant_ident.clone(),
                fields: Fields::Unit,
                discriminant: None,
            };

            enum_variants.push(enum_variant);

            let switch_stmt = if !is_unit_type {
                quote! {
                    self.#consume_name(postcard::from_bytes(payload_bytes)?);
                }
            } else {
                quote! { self.#consume_name(); }
            };

            deserialize_switch.push(quote! {
                #enum_ident::#variant_ident => {
                    #switch_stmt
                }
            });
            consume_methods.push(method);
        }

        if !field.attrs.iter().any(|attr| attr.path().is_ident("topic")) {
            cleaned_fields.push(field.clone());
        }
    }

    let attrs = vec![
        parse_quote! {
        #[derive(strum::EnumString)]},
        parse_quote! {#[strum(serialize_all = "snake_case")]},
    ];

    let dispatcher_enum = syn::ItemEnum {
        attrs,
        vis: Visibility::Inherited,
        enum_token: Enum {
            span: Span::call_site(),
        },
        ident: enum_ident.clone(),
        generics: Default::default(),
        brace_token: syn::token::Brace {
            ..Default::default()
        },
        variants: enum_variants,
    };

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

    let dispatcher = quote! {
        impl pusu::consumer::Consumer<#enum_ident> for #struct_name {
            fn dispatch(&self, topic: #enum_ident, payload_bytes: &[u8]) -> anyhow::Result<()> {
                match topic {
                    #(#deserialize_switch)*
                }
                Ok(())
            }
        }
    };

    let expanded = quote! {
        #dispatcher_enum

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
