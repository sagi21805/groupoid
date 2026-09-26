#![allow(dead_code)]
use groupoid_macros::group;

trait Testing {
    type Meta;
}

mod inner {
    pub struct StateA;
}

#[group(G)]
impl Testing for (inner::StateA,) {
    type Meta = u8;
}

fn main() {}
