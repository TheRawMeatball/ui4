use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Fields};

#[proc_macro_derive(Lens)]
pub fn my_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let lensed_ident = input.ident;
    let lens_vis = input.vis;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (inner_impls, outer_impls) = match input.data {
        syn::Data::Struct(s) => match s.fields {
            Fields::Named(fields) => fields
                .named
                .into_iter()
                .map(|field| (field.ident.unwrap(), field.ty))
                .map(|(ident, ty)| {
                    let outer = quote! {
                        #[derive(Copy, Clone)]
                        #[allow(non_snake_case)]
                        #lens_vis struct #ident;
                        impl ::ui4::lens::Lens for #ident {
                            type In = #lensed_ident;
                            type Out = #ty;

                            fn get<'a>(&self, v: &'a #lensed_ident) -> &'a #ty {
                                &v.#ident
                            }

                            fn get_mut<'a>(&self, v: &'a mut #lensed_ident) -> &'a mut #ty {
                                &mut v.#ident
                            }
                        }
                    };
                    let inner = quote! {
                        #lens_vis const #ident: #ident = #ident;
                    };
                    (inner, outer)
                })
                .unzip::<_, _, TokenStream, TokenStream>(),
            Fields::Unnamed(fields) => {
                let lens_name =
                    syn::Ident::new(&format!("{}Lens", lensed_ident), Span::call_site());
                let (inner_impls, outer_impls) = fields
                    .unnamed
                    .into_iter()
                    .map(|field| field.ty)
                    .enumerate()
                    .map(|(i, ty)| {
                        let index = syn::Index::from(i);
                        let outer = quote! {
                            impl ::ui4::lens::Lens for #lens_name<#i> {
                                type In = #lensed_ident;
                                type Out = #ty;

                                fn get<'a>(&self, v: &'a #lensed_ident) -> &'a #ty {
                                    &v.#index
                                }

                                fn get_mut<'a>(&self, v: &'a mut #lensed_ident) -> &'a mut #ty {
                                    &mut v.#index
                                }
                            }
                        };
                        let n_ident = syn::Ident::new(&format!("F{}", i), Span::call_site());
                        let inner = quote! {
                            #lens_vis const #n_ident: #lens_name::<#i> = #lens_name::<#i>;
                        };
                        (inner, outer)
                    })
                    .unzip::<_, _, TokenStream, TokenStream>();
                let outer = quote! {
                    #[derive(Copy, Clone)]
                    #lens_vis struct #lens_name<const N: usize>;
                    #outer_impls
                };
                let inner = quote! {
                    #inner_impls
                };
                (inner, outer)
            }
            Fields::Unit => unimplemented!("Unit structs not supported"),
        },
        _ => unimplemented!("Only structs are supported"),
    };

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        impl #impl_generics #lensed_ident #ty_generics #where_clause {
            #inner_impls
        }

        #outer_impls
    };

    // Hand the output tokens back to the compiler
    expanded.into()
}
