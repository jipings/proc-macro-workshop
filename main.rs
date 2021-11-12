use std::fmt;

// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run
struct GeekKindergarten {
    blog: String,
    ideawand: i32,
    com: bool,
}

impl fmt::Debug for GeekKindergarten {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("GeekKindergarten")
        .field("blog", &self.blog)
        .field("ideawand", &self.ideawand)
        .field("com", &self.com)
        .finish()
    }
}
fn main() {

    let g = GeekKindergarten{blog:"foo".into(), ideawand:123, com:true};
    println!("{:?}", g);
}
