use proc_macro::{TokenStream};
use proc_macro2;
use quote;
use syn;

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    // eprintln!("{:#?}", input);
    let st = syn::parse_macro_input!(input as SeqParser);

    let mut ret = proc_macro2::TokenStream::new();

    let buffer = syn::buffer::TokenBuffer::new2(st.body.clone());
    let (ret_1, expanded) = st.find_block_to_expand_and_do_expand(buffer.begin());
    // eprintln!("{:?}", expanded);
    if expanded {
        return ret_1.into();
    }

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

        let mut inc = false;
        if input.peek(syn::Token!(=)) {
            input.parse::<syn::Token!(=)>()?;
            inc = true;
        }

        // 假定`ParseStream`当前游标对应的是一个可以解析为整形数字面量的 Token，
        let end = input.parse::<syn::LitInt>()?;

        // 这里展示了 braced! 宏的用法，用于把一个代码块整体读取出来，如果读取成功就将代码块
        // 内部数据作为一个 `ParseBuffer` 类型的数据返回，同时把读取游标移动到整个代码块的后面
        let body_buf;
        syn::braced!(body_buf in input);
        let body: proc_macro2::TokenStream = body_buf.parse()?;

        let mut t = SeqParser {
            variable_ident,
            start: start.base10_parse()?,
            end: end.base10_parse()?,
            body,
        };
        if inc {
            t.end += 1;
        }
        Ok(t)
    }
}

impl SeqParser {
    fn expand(&self, ts: &proc_macro2::TokenStream, n: isize) -> proc_macro2::TokenStream {
        let buf = ts.clone().into_iter().collect::<Vec<_>>();
        let mut ret = proc_macro2::TokenStream::new();

        let mut idx = 0;
         while idx < buf.len() {
            let tree_node = &buf[idx];
            match tree_node {
                proc_macro2::TokenTree::Group(g) => {
                    // 如果是括号包含的内容，我们就递归处理内部的TokenStream
                    let new_stream = self.expand(&g.stream(), n);
                    // 这里需要注意，g.stream() 返回的是Group内部的TokenStream.
                    let wrap_in_group = proc_macro2::Group::new(g.delimiter(), new_stream);
                    ret.extend(quote::quote! {#wrap_in_group});
                }
                proc_macro2::TokenTree::Ident(prefix) => {
                    if idx + 2 < buf.len() { // 我们需要向后预读两个TokenTree元素
                        if let proc_macro2::TokenTree::Punct(p) = &buf[idx+1] {
                            if p.as_char() == '#' {
                                if let proc_macro2::TokenTree::Ident(i) = &buf[idx+2] {
                                    if i == &self.variable_ident // 校验是否连续，无空格
                                    && prefix.span().end() == p.span().start()
                                    && p.span().end() == i.span().start()
                                    {
                                        let new_ident_litral = format!("{}{}", prefix.to_string(), n);
                                        let new_ident = proc_macro2::Ident::new(new_ident_litral.as_str(), prefix.span());
                                        ret.extend(quote::quote!(#new_ident));
                                        idx += 3;
                                        continue;
                                    }

                                }
                            }
                        }

                    }


                    // 如果是一个 Ident，那么看一下是否为要替换的变量标识符，如果是则替换，如果不是则透传
                    if prefix == &self.variable_ident {
                        let new_ident = proc_macro2::Literal::i64_unsuffixed(n as i64);
                        ret.extend(quote::quote! {#new_ident});
                        idx += 1;
                        continue;
                    } 
                    ret.extend(quote::quote! {#tree_node});
                    
                }
                _ => {
                    // 对于其他的元素（也就是Punct和Literal），原封不动传递
                    ret.extend(quote::quote! {#tree_node});
                }
            }
            idx += 1;
        }

        ret
    }

    fn find_block_to_expand_and_do_expand(&self, c: syn::buffer::Cursor) -> (proc_macro2::TokenStream, bool) {
        let mut found = false;
        let mut ret = proc_macro2::TokenStream::new();
    
        let mut cursor = c;
        while !cursor.eof() {
            // 注意punct()这个函数的返回值，它返回一个新的 `Cursor` 类型的值
            // 这个新的 Cursor 指向了消耗当前标点符号以后，在TokenBuffer 中的下一个位置
            // syn包提供的Cursor机制，并不是拿到一个Cursor以后，不断向后移动更新这个Cursor，
            // 而是每次都会返回一个全新的Cursor，新的Cursor指向新的位置，老的Cursor指向的位置保持不变
            if let Some((punct_prefix, cursor_1)) = cursor.punct() {
                if punct_prefix.as_char() == '#' {
                    if let Some((group_cur,_,cursor_2)) = cursor_1.group(proc_macro2::Delimiter::Parenthesis) {
                        if let Some((punct_suffix, cursor_3)) = cursor_2.punct() {
                            if punct_suffix.as_char() == '*' {
                                // 找到了匹配的模式，按照指定的次数开始展开
                                for i in self.start..self.end {
                                    let t = self.expand(&group_cur.token_stream(), i);
                                    ret.extend(t);
                                }
                                cursor = cursor_3;
                                found = true;
                                continue;
                            }
                        }
                        
                    }
                }   
            }
    
            if let Some((group_cur,_,next_cur)) = cursor.group(proc_macro2::Delimiter::Brace) {
                let (t, f) = self.find_block_to_expand_and_do_expand(group_cur);
                found = f;
                ret.extend(quote::quote!({#t}));
                cursor = next_cur;
                continue;
            } else if let Some((group_cur,_,next_cur)) = cursor.group(proc_macro2::Delimiter::Bracket) {
                let (t, f) = self.find_block_to_expand_and_do_expand(group_cur);
                found = f;
                ret.extend(quote::quote!([#t]));
                cursor = next_cur;
                continue;
            } else if let Some((group_cur,_,next_cur)) = cursor.group(proc_macro2::Delimiter::Parenthesis) {
                let (t, f) = self.find_block_to_expand_and_do_expand(group_cur);
                found = f;
                ret.extend(quote::quote!((#t)));
                cursor = next_cur;
                continue;
            } else if let Some((punct, next_cur)) = cursor.punct() {
                ret.extend(quote::quote!(#punct));
                cursor = next_cur;
                continue;
            } else if let Some((ident, next_cur)) = cursor.ident() {
                ret.extend(quote::quote!(#ident));
                cursor = next_cur;
                continue;
            } else if let Some((literal, next_cur)) = cursor.literal() {
                ret.extend(quote::quote!(#literal));
                cursor = next_cur;
                continue;
            } else if let Some((lifetime, next_cur)) = cursor.lifetime() {
                ret.extend(quote::quote!(#lifetime));
                cursor = next_cur;
                continue;
            }
        }
        (ret, found)
    }
}
