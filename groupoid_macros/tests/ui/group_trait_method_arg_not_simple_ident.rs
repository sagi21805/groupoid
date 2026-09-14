#![allow(dead_code)]
use groupoid_macros::{blueprint, group_trait};

#[blueprint]
trait Testing {
    type Meta;
}

#[group_trait(by = Testing)]
trait Foo {
    fn combine(&self, (x, y): (i32, i32)) -> i32;
}

fn main() {}
