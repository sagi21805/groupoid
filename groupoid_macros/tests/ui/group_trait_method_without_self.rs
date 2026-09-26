#![allow(dead_code)]
use groupoid_macros::{group_trait, template};

#[template]
trait Testing {
    type Meta;
}

#[group_trait(by = Testing)]
trait Foo {
    fn make() -> Self;
}

fn main() {}
