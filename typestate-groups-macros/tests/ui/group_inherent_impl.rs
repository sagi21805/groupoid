#![allow(dead_code)]
use typestate_groups_macros::group;

struct Foo;

#[group(G)]
impl Foo {
    fn method() {}
}

fn main() {}
