#![allow(dead_code)]
use groupoid_macros::{group_trait, template};

#[template]
trait Testing {
    type Meta;
}

#[group_trait(by = Testing)]
trait Foo {
    fn combine(&self, (x, y): (i32, i32)) -> i32;
}

fn main() {}
