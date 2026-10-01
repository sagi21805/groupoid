#![allow(dead_code)]
use typestate_groups_macros::typestate;

#[typestate(state = X)]
struct Foo<S> {
    s: S,
}

fn main() {}
