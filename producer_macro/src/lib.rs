extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Field, FieldsNamed, Type, TypeTuple, parse_macro_input};

#[proc_macro_attribute]
pub fn producer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemStruct);
    let struct_name = &input.ident;

    let fields = if let syn::Fields::Named(f) = &input.fields {
        &f.named
    } else {
        panic!("Producer macro only supports named fields");
    };

    let mut produce_methods = Vec::new();
    let mut map_fields = syn::punctuated::Punctuated::new();
    let mut dispatcher_switches = Vec::new();

    
    for field in fields {
        let name = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        
        let produce_name = syn::Ident::new(&format!("produce_{}", name), name.span());
        
        let topic_lit = syn::LitStr::new(&name.to_string(), name.span());
        let topic_str_token = topic_lit.value();
        
        
        let (params_tokens, value_tokens) = if is_unit(ty) {
            (quote! { #topic_str_token, &() }, quote! {&mut self})
        } else {
            (quote! { #topic_str_token, &value }, quote! {&mut self, value: #ty})
        };
        
        let produce_method = quote! {
            fn #produce_name(#value_tokens) -> anyhow::Result<()> {
                self.#name.send(#params_tokens)
            }
        };
        
        let dispatcher_switch = quote! {
            #topic_str_token => self.#name.add_receiver(id, addr),
        };
        
        produce_methods.push(produce_method);
        dispatcher_switches.push(dispatcher_switch);
        
        let brokers_type: Type = syn::parse_str(&format!("pusu::producer::Receivers<{}>", ty.to_token_stream().to_string())).unwrap();

        let map_field = Field {
            attrs: Vec::new(),
            vis: syn::Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: field.ident.clone(),
            colon_token: None,
            ty: brokers_type,
        };
        map_fields.push(map_field);
    }

    let map_fields = syn::Fields::Named(FieldsNamed {
        brace_token: Default::default(),
        named: map_fields,
    });

    let mut attrs = input.attrs.clone();
    attrs.push(syn::parse_quote!(#[derive(Default)]));

    let output_struct = syn::ItemStruct {
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

    dispatcher_switches.push(quote! {
        _ => anyhow::bail!("Topic not found"),
    });

    let receiver_dispatch_trait = quote! {
        impl pusu::producer::ReceiverDispatch for #struct_name {
            fn add_receiver(&mut self, topic: &str, id: usize, addr: &str) -> Result<()> {
                match topic {
                    #(#dispatcher_switches)*
                }
                Ok(())
            }
        }
    };

    let expanded = quote! {
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