// Figure out what impl needs to be generated for the Debug impl of Field<T>.
// This will involve adding a trait bound to the T type parameter of the
// generated impl.
//
// Callers should be free to instantiate Field<T> with a type parameter T which
// does not implement Debug, but such a Field<T> will not fulfill the trait
// bounds of the generated Debug impl and so will not be printable via Debug.
//
//
// Resources:
//
//   - Representation of generics in the Syn syntax tree:
//     https://docs.rs/syn/1.0/syn/struct.Generics.html
//
//   - A helper for placing generics into an impl signature:
//     https://docs.rs/syn/1.0/syn/struct.Generics.html#method.split_for_impl
//
//   - Example code from Syn which deals with type parameters:
//     https://github.com/dtolnay/syn/tree/master/examples/heapsize

use derive_debug::CustomDebug;

#[derive(CustomDebug)]
pub struct Field<T> {
    value: T,
    #[debug = "0b{:08b}"]
    bitmask: u8,
}
// 1. 从DeriveInput语法树节点获取泛型参数信息
// 2. 为每一个泛型参数都添加一个Debug Trait限定
// 3. 使用split_for_impl()工具函数切分出用于模板生成代码的三个片段
// 4. 修改impl块的模板代码，使用上述三个片段，加入泛型参数信息

fn main() {
    let f = Field {
        value: "F",
        bitmask: 0b00011100,
    };

    let debug = format!("{:?}", f);
    let expected = r#"Field { value: "F", bitmask: 0b00011100 }"#;

    assert_eq!(debug, expected);
}
