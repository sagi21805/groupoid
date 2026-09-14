#![allow(dead_code)]
use groupoid_macros::{blueprint, group_trait};

#[blueprint]
trait Testing {
    type Meta;
}

#[group_trait(by = Testing)]
trait Foo {
    fn make() -> Self;
}

fn main() {}
