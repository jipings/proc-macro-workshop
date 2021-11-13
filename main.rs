use std::fmt::{self, Debug};

// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run
struct GeekKindergarten<T> {
    blog: T,
    ideawand: i32,
    com: bool,
}

impl<T> fmt::Debug for GeekKindergarten<T> 
where T: Debug
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("GeekKindergarten")
        .field("blog", &self.blog)
        .field("ideawand", &self.ideawand)
        .field("com", &self.com)
        .finish()
    }
}
fn main() {

    let g: GeekKindergarten<&str> = GeekKindergarten{blog: "foo".into(), ideawand:123, com:true};
    println!("{:?}", g);
    println!("{:?}", format_args!("0b{:32b}", 123));
}
