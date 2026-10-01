#![allow(dead_code)]
use typestate_groups_macros::group_impl;

struct Foo;

#[group_impl(G)]
impl Foo {
    fn method(&self) {}
}

fn main() {}
