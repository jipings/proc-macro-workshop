use std::collections::HashMap;

use proc_macro::TokenStream;
use syn::{self, parse_quote, visit::{self, Visit}};
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

    // 判定是否设置了限定条件干预，如果设定了，则不进行推断，直接使用用户给出的限定条件放到where子句中
    if let Some(hatch) = get_struct_escape_hatch(st) {
        generics_param_to_modify.make_where_clause();
        generics_param_to_modify
            .where_clause
            .as_mut()
            .unwrap()
            .predicates
            .push(syn::parse_str(hatch.as_str()).unwrap());
    } else {
        let fields = get_fields_from_derive_input(st)?;
        let mut field_type_names = Vec::new();
        let mut phantomdata_type_param_names = Vec::new();
        for field in fields {
            if let Some(s) = get_field_type_name(field)? {
                field_type_names.push(s);
            }
            if let Some(s) = get_phantomdata_generic_type_name(field)? {
                phantomdata_type_param_names.push(s);
            }
        }

        // 我们需要对每一个泛型参数都添加一个`Debug` Trait 限定
        let associated_types_map = get_generic_associated_types(st);
        for g in generics_param_to_modify.params.iter_mut() {
            if let syn::GenericParam::Type(t) = g {
                let type_param_name = t.ident.to_string();
                if phantomdata_type_param_names.contains(&type_param_name) && !field_type_names.contains(&type_param_name) {
                    continue;
                }
                // 如果是关联类型，就不要对泛型参数`T`本身再添加约束了,除非`T`本身也被直接使用了
                if associated_types_map.contains_key(&type_param_name) && !field_type_names.contains(&type_param_name){
                    continue
                }
                t.bounds.push(parse_quote!(std::fmt::Debug));
            }
        }

        // 关联类型的约束要放到where子句里
        generics_param_to_modify.make_where_clause();
        for (_, associated_types) in associated_types_map {
            for associated_type in associated_types {
                generics_param_to_modify.where_clause.as_mut().unwrap().predicates.push(parse_quote!(#associated_type:std::fmt::Debug));
            }
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

fn get_phantomdata_generic_type_name(field: &syn::Field) -> syn::Result<Option<String>> {
    if let syn::Type::Path(syn::TypePath{path: syn::Path{ref segments, ..}, ..}) = field.ty {
        if let Some(syn::PathSegment{ref ident, ref arguments}) = segments.last() {
            if ident == "PhantomData" {
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {args, ..}) = arguments {
                    if let Some(syn::GenericArgument::Type(syn::Type::Path(ref gp))) = args.first() {
                        if let Some(generic_ident) = gp.path.segments.first() {
                            return Ok(Some(generic_ident.ident.to_string()))
                        }
                    }
                }
            }
        }
    }
    return Ok(None);
}

fn get_field_type_name(field: &syn::Field) -> syn::Result<Option<String>> {
    if let syn::Type::Path(syn::TypePath{path: syn::Path{ ref segments, ..}, ..}) = field.ty {
        if let Some(syn::PathSegment {ref ident, ..}) = segments.last() {
            return Ok(Some(ident.to_string()));
        }
    }
    return Ok(None);
}

// 定义一个用于实现`Visit` Trait的结构体，结构体中定义了一些字段，用于存储筛选条件以及筛选结果
struct TypePathVisitor {
    generic_type_names: Vec<String>, // 这个是筛选条件，里面记录了所有的泛型参数的名字，例如`T`,`U`等
    associated_types: HashMap<String, Vec<syn::TypePath>>, // 这里记录了所有满足条件的语法树节点
}

impl <'ast> Visit<'ast> for TypePathVisitor {
    fn visit_type_path(&mut self, i: &'ast syn::TypePath) {
        
        if i.path.segments.len() >= 2 {
            let generic_type_name = i.path.segments[0].ident.to_string();
            if self.generic_type_names.contains(&generic_type_name) {
                // 如果满足上面的两个筛选条件，那么就把结果存起来
                self.associated_types.entry(generic_type_name).or_insert(Vec::new()).push(i.clone());
            }
        }
        // Visit 模式要求在当前节点访问完成后，继续调用默认实现的visit方法，从而遍历到所有的
        // 必须调用这个函数，否则遍历到这个节点就不再往更深层走了
        visit::visit_type_path(self, i);
    }
}

fn get_generic_associated_types(st: &syn::DeriveInput) -> HashMap<String, Vec<syn::TypePath>> {
    // 首先构建筛选条件
    let origin_generic_param_names: Vec<String> = st.generics.params.iter().filter_map(|f| {
        if let syn::GenericParam::Type(ty) = f {
            return Some(ty.ident.to_string());
        }
        None
    }).collect();

    let mut visitor = TypePathVisitor {
        generic_type_names: origin_generic_param_names,
        associated_types: HashMap::new(),
    };

    // 以st语法树节点为起点，开始Visit整个st节点的子节点
    visitor.visit_derive_input(st);
    return visitor.associated_types;
}

fn get_struct_escape_hatch(st: &syn::DeriveInput) -> Option<String> {
    if let Some(inner_attr) = st.attrs.last() {
        if let Ok(syn::Meta::List(syn::MetaList {nested, ..})) = inner_attr.parse_meta(){
            if let Some(syn::NestedMeta::Meta(syn::Meta::NameValue(path_value))) = nested.last() {
                if path_value.path.is_ident("bound") {
                    if let syn::Lit::Str(ref lit) = path_value.lit {
                        return Some(lit.value());
                    }
                }
            }
        }
    }
    None
}

