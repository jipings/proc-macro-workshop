use proc_macro2;
use syn::{self, parse_macro_input, spanned::Spanned };
use quote::{quote};
extern crate proc_macro;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_derive = parse_macro_input!(input as syn::DeriveInput);
    // eprint!("INPUT: {:#?}", input_derive.ident);
    // proc_macro::TokenStream::from(.unwrap())
    match do_expand(&input_derive) {
        Ok(token_stream) => token_stream.into(),
        Err(e) => e.to_compile_error().into(), 
    }
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

fn get_generic_inner_type<'a>(ty: &'a syn::Type, outer_ident_name: &str) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath { ref path, .. }) = ty {
        // 这里我们取segments的最后一节来判断是不是`T<U>`，这样如果用户写的是`foo:bar::T<U>`我们也能识别出最后的`T<U>`
        if let Some(seg) = path.segments.last() {
            if seg.ident == outer_ident_name {
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { 
                    ref args, 
                    ..
                }) = seg.arguments
                {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

fn generate_builder_struct_fields_def(fields: &StructFields) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| { &f.ident }).collect();
    let types: syn::Result<Vec<proc_macro2::TokenStream>> = fields
        .iter()
        .map(|f| {
            // 针对是否为 `Option` 类型字段，产生不同的结果
            if let Some(inner_ty) = get_generic_inner_type(&f.ty,"Option") {
                Ok(quote!(std::option::Option<#inner_ty>))
            } else if get_user_specified_ident_for_vec(f)?.is_some() {
                let origin_ty = &f.ty;
                Ok(quote!(#origin_ty)) 
            } else {
                let origin_ty = &f.ty;
               Ok(quote!(std::option::Option<#origin_ty>))
            }
        })
        .collect();

    let types = types?;
    let token_stream = quote! {
        #(#idents: #types),*
    };
    Ok(token_stream)
} 

fn generate_builder_struct_factory_init_clauses(fields: &StructFields) -> syn::Result<Vec<proc_macro2::TokenStream>>{
    let init_clauses: syn::Result<Vec<proc_macro2::TokenStream>> = fields.iter().map(|f| {
        let ident = &f.ident;
        if get_user_specified_ident_for_vec(f)?.is_some() {
            Ok(quote! {
                #ident: std::vec::Vec::new()
            })
        } else {
            Ok(quote!{
                #ident: std::option::Option::None
            })
        }
    }).collect();

    Ok(init_clauses?)
}

fn generate_setter_funcs(fields: &StructFields) -> syn::Result<proc_macro2::TokenStream> {
    let idents:Vec<_> = fields.iter().map(|f| {&f.ident}).collect();
    let types:Vec<_> = fields.iter().map(|f| {&f.ty}).collect();

    let mut final_tokenstream = proc_macro2::TokenStream::new();

    for (idx, (ident, ty)) in idents.iter().zip(types.iter()).enumerate() {
        let mut tokenstream_piece;
        if let Some(inner_ty) = get_generic_inner_type(ty, "Option") {
            tokenstream_piece = quote! {
                fn #ident(&mut self, #ident: #inner_ty) -> &mut Self {
                    self.#ident = std::option::Option::Some(#ident);
                    self
                }
            };
        } else if let Some(ref user_specified_ident) = get_user_specified_ident_for_vec(&fields[idx])? {
            let inner_ty = get_generic_inner_type(ty, "Vec")
                    .ok_or(syn::Error::new(fields[idx].span(),"each field must be specified with Vec field"))?;
            tokenstream_piece = quote! {
                fn #user_specified_ident(&mut self, #user_specified_ident: #inner_ty) -> &mut Self {
                    self.#ident.push(#user_specified_ident);
                    self
                }
            };
            // 如果用户指定的setter名字和原始字段的名字不一样，那么产生另一个setter，这个setter是一次性传入一个列表的
            if user_specified_ident != ident.as_ref().unwrap() {
                tokenstream_piece.extend(
                    quote! {
                        fn #ident(&mut self, #ident: #ty) -> &mut Self {
                            self.#ident = #ident.clone();
                            self
                        }
                    }
                );
            }

        } else {
            tokenstream_piece = quote! {
                fn #ident(&mut self, #ident: #ty) -> &mut Self {
                    self.#ident = std::option::Option::Some(#ident);
                    self
                }
            }
        }
        final_tokenstream.extend(tokenstream_piece);
    }
    Ok(final_tokenstream)
}

fn generate_build_function(fields: &StructFields, origin_struct_ident: &syn::Ident) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| {&f.ident}).collect();
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    let mut checker_code_pieces =Vec::new();
    let mut fill_result_clauses = Vec::new();

    for (idx, (ident, ty)) in idents.iter().zip(types.iter()).enumerate() {
        if get_generic_inner_type(ty, "Option").is_none() && get_user_specified_ident_for_vec(&fields[idx])?.is_none() {
            checker_code_pieces.push(quote! {
                if self.#ident.is_none() {
                    let err = format!("{} field missing", stringify!(#ident));
                    return std::result::Result::Err(err.into())
                }
            });
        }

        if get_user_specified_ident_for_vec(&fields[idx])?.is_some() {
            fill_result_clauses.push(quote! {
                #ident: self.#ident.clone()
            });
        } else if get_generic_inner_type(ty, "Option").is_none() {
            fill_result_clauses.push(quote!{
                #ident: self.#ident.clone().unwrap()
            });
        } else {
            fill_result_clauses.push(quote!{
                #ident: self.#ident.clone()
            });
        }
        
    }

    let token_stream = quote! {
        pub fn build(&mut self) -> std::result::Result<#origin_struct_ident, std::boxed::Box<dyn std::error::Error>> {
            #(#checker_code_pieces)* // 注意，由于我们要重复的是一组if判断代码块，它们之间不需要用逗号分隔，所以这里的重复模式是`*`，而不是之前重复结构体字段时用到的`,*`

            let ret = #origin_struct_ident {
                #(#fill_result_clauses),*
            };
            std::result::Result::Ok(ret)
        }
    };
    
    Ok(token_stream)
}

fn get_user_specified_ident_for_vec(field: &syn::Field) -> syn::Result<Option<syn::Ident>> {
    for attr in &field.attrs {
        if let Ok(syn::Meta::List(syn::MetaList {
            ref path,
            ref nested,
            ..
        })) = attr.parse_meta() 
        {
            if let Some(p) = path.segments.first() {
                if p.ident == "builder" {
                    if let Some(syn::NestedMeta::Meta(syn::Meta::NameValue(kv))) = nested.first() {
                        if kv.path.is_ident("each") {
                            if let syn::Lit::Str(ref ident_str) = kv.lit {
                                return Ok(Some(syn::Ident::new(
                                    ident_str.value().as_str(),
                                    attr.span(),
                                )));
                            }
                        } else {
                            if let Ok(syn::Meta::List(ref list)) = attr.parse_meta() {
                                return Err(syn::Error::new_spanned(list, r#"expected `builder(each = "...")`"#))
                            }
                        }
                    }
                }
            }
        }
        
    }
    Ok(None)
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

    let generated_builder_functions = generate_build_function(fields, struct_ident)?;

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
            #generated_builder_functions
        }


    };

    return Ok(ret);
}