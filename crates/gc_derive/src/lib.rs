#![feature(extend_one)]

use proc_macro_crate::FoundCrate;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{DeriveInput, Ident, parse_macro_input};

fn get_crate_name_of(name: &str, span: Span) -> Ident {
    let Ok(crate_name) = proc_macro_crate::crate_name(name) else {
        return Ident::new(name, span);
    };
    match crate_name {
        FoundCrate::Itself => Ident::new("crate", Span::call_site()),
        FoundCrate::Name(name) => Ident::new(&name, span),
    }
}

#[proc_macro_derive(Trace, attributes(ignore_trace))]
pub fn derive_trace(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let gc_name = get_crate_name_of("pure_lang_gc", Span::call_site());
    let ast = parse_macro_input!(input as DeriveInput);
    let ty_name = &ast.ident;
    let (i_g, g, w) = ast.generics.split_for_impl();
    let ts = match &ast.data {
        syn::Data::Struct(data_struct) => {
            let mut ts = TokenStream::new();
            for (i, field) in data_struct.fields.iter().enumerate() {
                if field.attrs.iter().any(|x| {
                    x.path()
                        .get_ident()
                        .map(|y| y.eq("ignore_trace"))
                        .unwrap_or(false)
                }) {
                    continue;
                }
                match &field.ident {
                    Some(name) => ts.extend_one(quote! {
                        result.extend(#gc_name ::Trace::trace(&self. #name));
                    }),
                    None => ts.extend_one(quote! {
                        result.extend(#gc_name ::Trace::trace(&self. #i));
                    }),
                }
            }
            ts
        }
        syn::Data::Enum(data_enum) => {
            let mut ts = TokenStream::new();
            ts.extend_one(quote!(match self));
            let mut holder = TokenStream::new();
            for variant in &data_enum.variants {
                if variant.attrs.iter().any(|x| {
                    x.path()
                        .get_ident()
                        .map(|y| y.eq("ignore_trace"))
                        .unwrap_or(false)
                }) {
                    continue;
                }
                let name = &variant.ident;
                holder.extend_one(quote!(Self:: #name));
                match &variant.fields {
                    syn::Fields::Named(fields_named) => {
                        let fields = &fields_named.named;
                        for field in fields {
                            if field.attrs.iter().any(|x| {
                                x.path()
                                    .get_ident()
                                    .map(|y| y.eq("ignore_trace"))
                                    .unwrap_or(false)
                            }) {
                                continue;
                            }
                            match &field.ident {
                                Some(name) => holder.extend_one(quote! {
                                    result.extend(#gc_name ::Trace::trace(&self. #name));
                                }),
                                None => panic!(),
                            }
                        }
                    }
                    syn::Fields::Unnamed(fields_unnamed) => {
                        let fields = &fields_unnamed.unnamed;
                        let identifiers: Vec<_> = fields
                            .iter()
                            .enumerate()
                            .map(|(i, x)| {
                                if x.attrs.iter().any(|x| {
                                    x.path()
                                        .get_ident()
                                        .map(|y| y.eq("ignore_trace"))
                                        .unwrap_or(false)
                                }) {
                                    Ident::new("_", Span::call_site())
                                } else {
                                    format_ident!("_{}", i)
                                }
                            })
                            .collect();
                        for field in fields.iter() {
                            if field.attrs.iter().any(|x| {
                                x.path()
                                    .get_ident()
                                    .map(|y| y.eq("ignore_trace"))
                                    .unwrap_or(false)
                            }) {
                                continue;
                            }
                            holder.extend_one(quote! {
                                (#(#identifiers,)*)=> {#(result.extend(#gc_name ::Trace::trace(#identifiers));)*}
                            });
                        }
                    }
                    syn::Fields::Unit => holder.extend_one(quote!(=> (),)),
                }
            }
            ts.extend_one(quote!({#holder}));
            ts
        }
        syn::Data::Union(_data_union) => todo!(),
    };
    quote! {
        #[automatically_derived]
        impl #i_g #gc_name ::Trace for #ty_name #g #w {
            fn trace(&self) -> Vec<usize> {
                let mut result = Vec::new();
                #ts
                result
            }
        }
    }
    .into()
}
