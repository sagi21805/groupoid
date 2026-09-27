// `morph_with` converts an opaque type through one projection, so one
// that mentions two is rejected.
#![allow(dead_code)]
use groupoid_macros::{template, typestate};

#[template]
trait Meta {
    type Type1;
    type Type2;
}

#[typestate(state = S)]
struct Wrap<S: Meta> {
    a: S::Type1,
    convert: fn(S::Type1) -> S::Type2,
}

fn main() {}
