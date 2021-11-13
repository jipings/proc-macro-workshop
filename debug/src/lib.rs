use proc_macro::TokenStream;
use syn::{self, parse_quote};
use quote::{quote};

#[proc_macro_derive(CustomDebug, attributes(debug))]
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

fn generate_debug_trait_core(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = get_fields_from_derive_input(st)?;
    let struct_name_ident = &st.ident;
    let struct_name_str = struct_name_ident.to_string();
    let mut fmt_body_stream = proc_macro2::TokenStream::new();

    fmt_body_stream.extend(quote!(
        fmt.debug_struct(#struct_name_str)
    ));
    for field in fields.iter() {
        let field_name_ident = field.ident.as_ref().unwrap();
        let field_name_str = field_name_ident.to_string();
        
        let mut format_str = "{:?}".to_string();
        if let Some(format) = get_custom_format_of_field(field)? {
            format_str = format;
        }

        fmt_body_stream.extend(quote!(
            .field(#field_name_str, &format_args!(#format_str, self.#field_name_ident))
        ));
    }

    fmt_body_stream.extend(quote!(.finish()));
    return Ok(fmt_body_stream);
}

fn generate_debug_trait(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {

    let fmt_body_stream = generate_debug_trait_core(st)?;
    let struct_name_ident = &st.ident;

    // 从输入的派生宏语法树节点获取被修饰的输入结构体的泛型信息
    let mut generics_param_to_modify = st.generics.clone();
    // 我们需要对每一个泛型参数都添加一个`Debug` Trait 限定
    for g in generics_param_to_modify.params.iter_mut() {
        if let syn::GenericParam::Type(t) = g {
            t.bounds.push(parse_quote!(std::fmt::Debug));
        }
    }

    // 使用工具函数把泛型抽成3个片段
    let (impl_generics, type_generics, where_clause) = generics_param_to_modify.split_for_impl();

    let ret_stream = quote! {
        impl #impl_generics std::fmt::Debug for #struct_name_ident #type_generics #where_clause {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                #fmt_body_stream
            }
        }
    };

    return Ok(ret_stream);
}

fn get_custom_format_of_field(field: &syn::Field) -> syn::Result<Option<String>> {
    for attr in &field.attrs {
        if let Ok(syn::Meta::NameValue(syn::MetaNameValue {
            ref path,
            ref lit,
            ..
        })) = attr.parse_meta()
        {
            if path.is_ident("debug") {
                if let syn::Lit::Str(ref ident_str) = lit {
                    return Ok(Some(ident_str.value()));
                }
            }
        }
    }
    Ok(None)
}