// An associated type on a `#[group_trait]` trait can't be satisfied by
// the blanket impl.
#![allow(dead_code)]
use groupoid_macros::{blueprint, group_trait};

#[blueprint]
trait Testing {
    type Meta;
}

#[group_trait(by = Testing)]
trait Foo {
    type Extra;

    fn a(&self) -> Self::Extra;
}

fn main() {}
