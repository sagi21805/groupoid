#![allow(dead_code)]
use typestate_groups_macros::{group_trait, state_types};

#[state_types]
trait Testing {
    type Meta;
}

#[group_trait(by = Testing)]
trait Foo {
    fn combine(&self, (x, y): (i32, i32)) -> i32;
}

fn main() {}
