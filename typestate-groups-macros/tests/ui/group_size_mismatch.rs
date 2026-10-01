// `#[size(N)]` fails to compile when the type is not N bytes.
#![allow(dead_code)]
use typestate_groups_macros::{group, state, state_types};

#[state_types]
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
