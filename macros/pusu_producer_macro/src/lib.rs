extern crate proc_macro;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Field, Fields, FieldsNamed, Ident, ItemEnum, ItemStruct, Type, Variant, Visibility,
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    token::{Brace, Comma, Enum},
};

#[proc_macro_attribute]
pub fn producer(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let struct_name = &input.ident;

    let fields = if let Fields::Named(f) = &input.fields {
        &f.named
    } else {
        panic!("Producer macro only supports named fields");
    };

    let mut topic_fields = Vec::new();
    let mut struct_fields = Punctuated::<Field, Comma>::new();

    for field in fields {
        let ty = &field.ty;

        // let mut is_topic = false;
        let mut field_attrs = Vec::new();

        for attr in field.attrs.clone() {
            // if attr.path().is_ident("topic") {
            // is_topic = true;
            // } else {
            field_attrs.push(attr);
            // }
        }

        let new_field = Field {
            attrs: field_attrs,
            vis: Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: field.ident.clone(),
            colon_token: None,
            ty: ty.clone(),
        };

        // if is_topic {
        topic_fields.push(new_field.clone());
        // } else {
        // struct_fields.push(new_field);
        // }
    }

    let enum_variants: Vec<Ident> = topic_fields
        .iter()
        .map(|field| {
            Ident::new(
                &field
                    .ident
                    .as_ref()
                    .cloned()
                    .unwrap()
                    .to_string()
                    .to_case(Case::Pascal),
                Span::call_site(),
            )
        })
        .collect();

    let output_enum = generate_enum(struct_name, enum_variants);
    let enum_name = &output_enum.ident;
    let mut init_topics_param = Vec::new();

    let produce_functions = topic_fields.iter().enumerate().map(|(index, field)| {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        let enum_variant = &output_enum.variants[index].ident;
        init_topics_param.push(quote! { #enum_name::#enum_variant });
        quote! {
            pub fn #ident(&self, payload: &#ty) -> Result<(), pusu::producer::ProducerError> {
                self.manager.send(#enum_name::#enum_variant, payload)
            }
        }
    });

    let manager_field = Field {
        attrs: Vec::new(),
        vis: Visibility::Inherited,
        mutability: syn::FieldMutability::None,
        ident: Some(Ident::new("manager", Span::call_site())),
        colon_token: None,
        ty: Type::Verbatim(quote! {std::sync::Arc<pusu::producer::ProducerManager<#enum_name>>}),
    };

    let state_handle_field = Field {
        attrs: Vec::new(),
        vis: Visibility::Inherited,
        mutability: syn::FieldMutability::None,
        ident: Some(Ident::new("state_handle", Span::call_site())),
        colon_token: None,
        ty: Type::Verbatim(quote! {Option<pusu::producer::StateHandle>}),
    };

    struct_fields.push(manager_field);
    struct_fields.push(state_handle_field);

    let output_struct_fields = Fields::Named(FieldsNamed {
        brace_token: Default::default(),
        named: struct_fields,
    });

    let output_struct = ItemStruct {
        attrs: input.attrs.clone(),
        vis: input.vis,
        struct_token: input.struct_token,
        ident: input.ident.clone(),
        generics: input.generics,
        fields: output_struct_fields,
        semi_token: input.semi_token,
    };

    let expanded = quote! {
        #output_struct

        impl #struct_name {
            fn new(id: usize) -> Self {
                Self { manager: std::sync::Arc::new(pusu::producer::ProducerManager::new(id)), state_handle: Default::default() }
            }
        }

        impl #struct_name {
            #(#produce_functions)*

            pub fn from_config(filename: &str) -> Result<Self> where Self: Sized {
                let format = pusu::producer::config::get_file_format(filename)?;
                let config = pusu::producer::config::load_config(filename, format)?;
                let manager = pusu::producer::ProducerManager::from(config);
                let res = Self { manager: std::sync::Arc::new(manager), state_handle: Default::default() };
                Ok(res)
            }

            pub fn run(&mut self) -> Result<()> {
                let manager_arc = self.manager.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                let handle = std::thread::spawn(move || {
                    let _ = manager_arc.run(rx);
                });
                let state_handle = pusu::producer::StateHandle::new(tx, handle);
                self.state_handle = Some(state_handle);
                Ok(())
            }

            pub fn close(&mut self) -> Result<()> {
                if let Some(handle) = self.state_handle.take() {
                    handle.close()?;
                }
                Ok(())
            }

            pub fn add_receiver(&mut self, id: usize, addr: std::net::SocketAddr, topics: Vec<#enum_name>) {
                self.manager.add_receiver(id, addr, topics);
            }
        }

        impl pusu::producer::TopicContainer for #enum_name {
            fn get_topics() -> Vec<Self> {
                vec![#(#init_topics_param),*]
            }
        }

        #output_enum
    };

    TokenStream::from(expanded)
}

fn generate_enum(struct_name: &Ident, variants_ident: Vec<Ident>) -> ItemEnum {
    let enum_name = format!("{}Topic", struct_name);
    let enum_ident = Ident::new(&enum_name, Span::call_site());

    let variants = variants_ident
        .into_iter()
        .map(|variant_ident| Variant {
            attrs: Vec::new(),
            ident: variant_ident,
            fields: Fields::Unit,
            discriminant: None,
        })
        .collect();

    let attrs = vec![
        parse_quote! {#[derive(strum::EnumString, strum::Display, serde::Serialize, serde::Deserialize, Debug, Hash, PartialEq, Eq, Copy, Clone)]},
        parse_quote! {#[strum(serialize_all = "snake_case")]},
    ];

    ItemEnum {
        attrs,
        vis: Visibility::Inherited,
        enum_token: Enum {
            span: Span::call_site(),
        },
        ident: enum_ident.clone(),
        generics: Default::default(),
        brace_token: Brace {
            ..Default::default()
        },
        variants,
    }
}

// fn is_unit(ty: &syn::Type) -> bool {
//     matches!(ty, syn::Type::Tuple(t) if t.elems.is_empty())
// }

// fn extract_topic_inner_type(ty: &Type) -> Option<&Type> {
//     match ty {
//         Type::Path(type_path) => {
//             let segment = type_path.path.segments.last()?;

//             if segment.ident != "Topic" {
//                 return None;
//             }

//             match &segment.arguments {
//                 PathArguments::AngleBracketed(args) => {
//                     if args.args.len() != 1 {
//                         return None;
//                     }

//                     match args.args.first()? {
//                         GenericArgument::Type(inner_ty) => Some(inner_ty),
//                         _ => None,
//                     }
//                 }
//                 _ => None,
//             }
//         }
//         _ => None,
//     }
// }

// fn is_topic(ty: &Type) -> bool {
//     matches!(ty, Type::Path(type_path) if type_path.path.segments.last().map(|segment| segment.ident == "Topic").unwrap_or(false))
// }
