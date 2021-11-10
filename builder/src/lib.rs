use syn::{self, parse_macro_input};
extern crate proc_macro;

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_derive = parse_macro_input!(input as syn::DeriveInput);
    eprint!("INPUT: {:#?}", input_derive.ident);
    proc_macro::TokenStream::new()
}

