#![allow(dead_code)]
use groupoid_macros::typestate;

#[typestate(state = X)]
struct Foo<S> {
    s: S,
}

fn main() {}
