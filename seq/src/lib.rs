use proc_macro::{TokenStream};
use proc_macro2;
use syn;

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    // eprintln!("{:#?}", input);
    let st = syn::parse_macro_input!(input as SeqParser);

    let mut ret = proc_macro2::TokenStream::new();
    for i in st.start..st.end {
        ret.extend(st.expand(&st.body, i))
    }
    return ret.into();
}

struct SeqParser {
    variable_ident: syn::Ident,
    start: isize,
    end: isize,
    body: proc_macro2::TokenStream,
}

impl syn::parse::Parse for SeqParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // 我们要解析形 如 `N in 0..512 { ... }` 这样的代码片段
        // 假定`ParseStream` 当前游标对应的是一个可以解析为 `Ident` 类型的Token.
        // 如果是 `Ident` 类型节点，则返回Ok并将当前读取游标向后移动一个Token
        // 如果不是 `Ident` 类型，则返回 Err，说明语法错误 
        let variable_ident = input.parse::<syn::Ident>()?;
        // 假定`ParseStream`当前游标对应的是一个写作`in`的自定义的Token
        input.parse::<syn::Token!(in)>()?;
        // 假定`ParseStream`当前游标对应的是一个可以解析为整形字面量的Token.
        let start = input.parse::<syn::LitInt>()?;
        // 假定`ParseStream`当前游标对应的是一个写作`..`的自定义的Token
        input.parse::<syn::Token!(..)>()?;
        // 假定`ParseStream`当前游标对应的是一个可以解析为整形数字面量的 Token，
        let end = input.parse::<syn::LitInt>()?;

        // 这里展示了 braced! 宏的用法，用于把一个代码块整体读取出来，如果读取成功就将代码块
        // 内部数据作为一个 `ParseBuffer` 类型的数据返回，同时把读取游标移动到整个代码块的后面
        let body_buf;
        syn::braced!(body_buf in input);
        let body: proc_macro2::TokenStream = body_buf.parse()?;

        Ok(SeqParser {
            variable_ident,
            start: start.base10_parse()?,
            end: end.base10_parse()?,
            body,
        })
    }
}

impl SeqParser {
    fn expand(&self, ts: &proc_macro2::TokenStream, n: isize) -> proc_macro2::TokenStream {
        let buf = ts.clone().into_iter().collect::<Vec<_>>();
        let mut ret = proc_macro2::TokenStream::new();

        // 为了简单实用了for循环
        for idx in 0..buf.len() {
            let tree_node = &buf[idx];
            match tree_node {
                proc_macro2::TokenTree::Group(g) => {
                    // 如果是括号包含的内容，我们就递归处理内部的TokenStream
                    let new_stream = self.expand(&g.stream(), n);
                    // 这里需要注意，g.stream() 返回的是Group内部的TokenStream.
                    let wrap_in_group = proc_macro2::Group::new(g.delimiter(), new_stream);
                    ret.extend(quote::quote! {#wrap_in_group});
                }
                proc_macro2::TokenTree::Ident(i) => {
                    // 如果是一个 Ident，那么看一下是否为要替换的变量标识符，如果是则替换，如果不是则透传
                    if i == &self.variable_ident {
                        let new_ident = proc_macro2::Literal::i64_unsuffixed(n as i64);
                        ret.extend(quote::quote! {#new_ident});
                    } else {
                        ret.extend(quote::quote! {#tree_node});
                    }
                }
                _ => {
                    // 对于其他的元素（也就是Punct和Literal），原封不动传递
                    ret.extend(quote::quote! {#tree_node});
                }
            }
        }

        ret
    }

    
}