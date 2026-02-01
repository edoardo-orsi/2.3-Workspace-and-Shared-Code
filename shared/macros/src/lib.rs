mod bridge_to;
use proc_macro::TokenStream;

#[proc_macro_derive(ExposeStructure, attributes(bridge))]
pub fn expose_structure(input: TokenStream) -> TokenStream {
    bridge_to::implement_structure_exporter(input)
}
