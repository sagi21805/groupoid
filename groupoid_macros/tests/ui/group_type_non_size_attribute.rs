// Documents that an associated type inside a #[group] impl can carry
// `#[size(N)]` and nothing else - any other attribute is rejected rather
// than silently passed through.
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
    #[allow(dead_code)]
    type Meta = u64;
}

fn main() {}
