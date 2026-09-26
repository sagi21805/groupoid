#![allow(dead_code)]
use groupoid_macros::group_impl;

struct Foo;

#[group_impl(G)]
impl Foo {
    fn method(&self) {}
}

fn main() {}
