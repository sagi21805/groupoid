#![allow(dead_code)]
use groupoid_macros::{group, state};

mod inner {
    pub trait Testing {
        type Meta;
    }
}

#[state]
struct StateA;

#[group(G)]
impl inner::Testing for (StateA,) {
    type Meta = u8;
}

fn main() {}
