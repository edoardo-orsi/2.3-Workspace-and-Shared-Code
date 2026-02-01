use proc_macro::TokenStream;
use proc_macro2::{Punct, Spacing};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn implement_structure_exporter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let macro_name = format_ident!("propagate_{}", name.to_string().to_lowercase());

    let mut variants_code = Vec::new();
    let mut seen_types = HashSet::new();

    if let Data::Enum(data) = input.data {
        for variant in data.variants {
            let variant_name = &variant.ident;
            if let Fields::Unnamed(f) = &variant.fields {
                if let Some(field) = f.unnamed.first() {
                    let ty = &field.ty;
                    let type_string = quote!(#ty).to_string();

                    // SKIP duplicate types (like multiple Strings) to avoid compilation errors
                    if seen_types.contains(&type_string) {
                        continue;
                    }
                    seen_types.insert(type_string);

                    // Use Spacing::Joint to ensure $target and $bridge_variant work correctly
                    let d = Punct::new('$', Spacing::Joint);

                    variants_code.push(quote! {
                        impl From<#ty> for #d target {
                            fn from(err: #ty) -> Self {
                                #d target :: #d bridge_variant ( #name :: #variant_name ( err.into() ) )
                            }
                        }
                    });
                }
            }
        }
    }

    // This d is for the macro definition line
    let d = Punct::new('$', Spacing::Joint);

    let expanded = quote! {
        #[macro_export]
        macro_rules! #macro_name {
            (#d target:ident, #d bridge_variant:ident) => {
                #(#variants_code)*
            };
        }
    };

    TokenStream::from(expanded)
}
