use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn implement_structure_exporter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident; // The source Enum name (e.g., CommonError)

    // Generates a generic name
    let macro_name = format_ident!("propagate_{}", name.to_string().to_lowercase());

    let mut variants_code = Vec::new();

    if let Data::Enum(data) = input.data {
        for variant in data.variants {
            let variant_name = &variant.ident;
            if let Fields::Unnamed(f) = &variant.fields {
                if let Some(field) = f.unnamed.first() {
                    let ty = &field.ty;

                    variants_code.push(quote! {
                        impl From<#ty> for $target {
                            fn from(err: #ty) -> Self {
                                // $target is the destination enum (e.g. GatewayError)
                                // $bridge_variant is the wrapper (e.g. GatewayError::Common)
                                $target::$bridge_variant(#name::#variant_name(err.into()))
                            }
                        }
                    });
                }
            }
        }
    }

    let d = format_ident!("$"); // The dollar-sign escape trick

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
