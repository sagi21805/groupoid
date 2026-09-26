// Documents that `#[typestate]` with no `state = <Ident>` argument requires
// the struct to have exactly one generic type parameter to infer from.
#![allow(dead_code)]
use groupoid_macros::typestate;

#[typestate]
struct Foo<A, B> {
    a: A,
    b: B,
}

fn main() {}
