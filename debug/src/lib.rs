use proc_macro::TokenStream;
use syn::{self, spanned::Spanned};
use quote::{ToTokens, quote};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input_derive = syn::parse_macro_input!(input as syn::DeriveInput);
    match do_expand(&input_derive) {
        Ok(token_stream) => token_stream.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn do_expand(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ret = generate_debug_trait(st)?;

    Ok(ret)
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

fn generate_debug_trait(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = get_fields_from_derive_input(st)?;
    let struct_name_ident = &st.ident;
    let struct_name_str = struct_name_ident.to_string();

    let mut fmt_body_stream = proc_macro2::TokenStream::new();

    fmt_body_stream.extend(quote!(
        fmt.debug_struct(#struct_name_str) // 注意这里引用的是一个字符串，不是一个syn::Ident，生成的代码会以字面量形式表示出来
    ));
    for field in fields.iter() {
        let field_name_ident = field.ident.as_ref().unwrap();
        let field_name_str = field_name_ident.to_string();

        fmt_body_stream.extend(quote!(
            .field(#field_name_str, &self.#field_name_ident) // 这行同样注意
        ));
    }

    fmt_body_stream.extend(quote!(.finish()));

    let ret_stream = quote!{
        impl std::fmt::Debug for #struct_name_ident {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                #fmt_body_stream
            }
        }
    };

    return Ok(ret_stream);
}