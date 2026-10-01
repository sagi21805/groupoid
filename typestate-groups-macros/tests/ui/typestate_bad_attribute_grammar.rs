#![allow(dead_code)]
use typestate_groups_macros::typestate;

#[typestate(State = S)]
struct Foo<S> {
    s: S,
}

fn main() {}
