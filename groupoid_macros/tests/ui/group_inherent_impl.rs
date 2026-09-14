#![allow(dead_code)]
use groupoid_macros::group;

struct Foo;

#[group(G)]
impl Foo {
    fn method() {}
}

fn main() {}
