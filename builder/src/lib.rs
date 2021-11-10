use proc_macro2;
use syn::{self, parse_macro_input, spanned::Spanned };
use quote::{quote};
extern crate proc_macro;

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_derive = parse_macro_input!(input as syn::DeriveInput);
    // eprint!("INPUT: {:#?}", input_derive.ident);
    proc_macro::TokenStream::from(do_expand(&input_derive).unwrap())
}

type StructFields = syn::punctuated::Punctuated<syn::Field, syn::Token!(,)>;

fn get_fields_from_derive_input(d: &syn::DeriveInput) -> syn::Result<&StructFields> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = d.data {
        return Ok(named);
    }
    Err(syn::Error::new_spanned(d, "Must define on a Struct, not Enum".to_string()))
}

fn generate_builder_struct_fields_def(fields: &StructFields) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| { &f.ident }).collect();
    let types: Vec<_> = fields.iter().map(|f| { &f.ty }).collect();

    let token_stream = quote!{
        #(#idents: std::option::Option<#types>),*
    };
    Ok(token_stream)
} 

fn generate_builder_struct_factory_init_clauses(fields: &StructFields) -> syn::Result<Vec<proc_macro2::TokenStream>>{
    let init_clauses: Vec<_> = fields.iter().map(|f| {
        let ident = &f.ident;
        quote!{
            #ident: std::option::Option::None
        }
    }).collect();

    Ok(init_clauses)
}

fn generate_setter_funcs(fields: &StructFields) -> syn::Result<proc_macro2::TokenStream> {
    let idents:Vec<_> = fields.iter().map(|f| {&f.ident}).collect();
    let types:Vec<_> = fields.iter().map(|f| {&f.ty}).collect();

    let mut final_tokenstream = proc_macro2::TokenStream::new();

    for (ident, ty) in idents.iter().zip(types.iter()) {
        let tokenstream_piece = quote! {
            fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = std::option::Option::Some(#ident);
                self
            }
        };
        final_tokenstream.extend(tokenstream_piece);
    }
    Ok(final_tokenstream)
}

fn do_expand(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    // eprintln!("{:#?}", st.data);
    let struct_name = st.ident.to_string();
    let builder_name = format!("{}Builder", struct_name);
    let builder_name_ident = syn::Ident::new(&builder_name, st.span());

    let struct_ident = &st.ident;

    let fields = get_fields_from_derive_input(st)?;
    let builder_struct_fields_def = generate_builder_struct_fields_def(fields)?;

    let builder_struct_factory_init_clauses = generate_builder_struct_factory_init_clauses(fields)?;

    let builder_struct_funcs= generate_setter_funcs(fields)?;

    let ret = quote! {
        pub struct #builder_name_ident {
            #builder_struct_fields_def
        }
        impl #struct_ident {
            pub fn builder() -> #builder_name_ident{
                #builder_name_ident {
                    #(#builder_struct_factory_init_clauses),*
                }
            }
        }

        impl #builder_name_ident {
            #builder_struct_funcs
        }


    };

    return Ok(ret);
}