#![allow(dead_code)]
use groupoid_macros::typestate;

#[typestate(State = S)]
struct Foo<S> {
    s: S,
}

fn main() {}
