#![allow(dead_code)]
use groupoid_macros::{group, state};

trait Testing {
    type Meta;
}

#[state]
struct StateA;

#[group(G)]
impl Testing for (&StateA,) {
    type Meta = u8;
}

fn main() {}
