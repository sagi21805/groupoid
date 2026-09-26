// `#[size(N)]` fails to compile when the type is not N bytes.
#![allow(dead_code)]
use groupoid_macros::{blueprint, group, state};

#[blueprint]
trait Testing {
    type Meta;
}

#[state]
struct StateA;

#[group(G)]
impl Testing for (StateA,) {
    #[size(4)]
    type Meta = u64;
}

fn main() {}
