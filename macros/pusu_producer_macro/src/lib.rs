extern crate proc_macro;

use convert_case::Casing;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    Field, Fields, FieldsNamed, Ident, ItemStruct, LitStr, Type, TypeTuple, Variant, Visibility,
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    token::{Brace, Comma, Enum},
};

#[proc_macro_attribute]
pub fn producer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;

    let fields = if let Fields::Named(f) = &input.fields {
        &f.named
    } else {
        panic!("Producer macro only supports named fields");
    };

    let mut produce_methods = Vec::new();
    let mut map_fields = Punctuated::new();
    let mut dispatcher_switches = Vec::new();
    let mut enum_variants = Punctuated::<Variant, Comma>::new();

    let enum_name = format!("{}Topic", struct_name);
    let enum_ident = Ident::new(&enum_name, Span::call_site());

    for field in fields {
        let name = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        let produce_name = Ident::new(&format!("produce_{}", name), name.span());

        let topic_lit = LitStr::new(&name.to_string(), name.span());
        let topic_str = topic_lit.value();

        let (params_tokens, value_tokens) = if is_unit(ty) {
            (quote! { #topic_str, &() }, quote! {&mut self})
        } else {
            (
                quote! { #topic_str, &value },
                quote! {&mut self, value: #ty},
            )
        };

        let produce_method = quote! {
            fn #produce_name(#value_tokens) -> anyhow::Result<()> {
                self.#name.send(#params_tokens)
            }
        };

        let variant_ident = Ident::new(
            &topic_str.to_case(convert_case::Case::Pascal),
            Span::call_site(),
        );
        let enum_variant = Variant {
            attrs: Vec::new(),
            ident: variant_ident.clone(),
            fields: Fields::Unit,
            discriminant: None,
        };

        let dispatcher_switch = quote! {
            #enum_ident::#variant_ident => self.#name.add_receiver(id, addr),
        };

        produce_methods.push(produce_method);
        dispatcher_switches.push(dispatcher_switch);
        enum_variants.push(enum_variant);

        let brokers_type: Type = syn::parse_str(&format!(
            "pusu::producer::Receivers<{}>",
            ty.to_token_stream()
        ))
        .unwrap();

        let map_field = Field {
            attrs: Vec::new(),
            vis: Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: field.ident.clone(),
            colon_token: None,
            ty: brokers_type,
        };
        map_fields.push(map_field);
    }

    let map_fields = Fields::Named(FieldsNamed {
        brace_token: Default::default(),
        named: map_fields,
    });

    let enum_attrs = vec![
        parse_quote! {
        #[derive(strum::EnumString)]},
        parse_quote! {#[strum(serialize_all = "snake_case")]},
    ];

    let dispatcher_enum = syn::ItemEnum {
        attrs: enum_attrs,
        vis: Visibility::Inherited,
        enum_token: Enum {
            span: Span::call_site(),
        },
        ident: enum_ident.clone(),
        generics: Default::default(),
        brace_token: Brace {
            ..Default::default()
        },
        variants: enum_variants,
    };

    let mut attrs = input.attrs.clone();
    attrs.push(parse_quote!(#[derive(Default)]));

    let output_struct = ItemStruct {
        attrs,
        vis: input.vis,
        struct_token: input.struct_token,
        ident: input.ident.clone(),
        generics: input.generics,
        fields: map_fields,
        semi_token: input.semi_token,
    };

    let new_method = quote! {
        pub fn new() -> Self {
            Self::default()
        }
    };

    let receiver_dispatch_trait = quote! {
        impl pusu::producer::ReceiverDispatch<#enum_ident> for #struct_name {
            fn add_receiver(&mut self, topic: #enum_ident, id: usize, addr: &str) {
                match topic {
                    #(#dispatcher_switches)*
                }
            }
        }
    };

    let expanded = quote! {
        #dispatcher_enum

        #output_struct

        impl #struct_name {
            #new_method

            #(#produce_methods)*
        }

        #receiver_dispatch_trait
    };

    TokenStream::from(expanded)
}

fn is_unit(ty: &Type) -> bool {
    match ty {
        Type::Tuple(TypeTuple { elems, .. }) => elems.is_empty(),
        _ => false,
    }
}
