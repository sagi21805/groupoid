// `#[group]` on a trait without `#[state_types]` fails to find the marker
// trait.
#![allow(dead_code)]
use typestate_groups_macros::{group, state};

trait Testing {
    type Meta;
}

#[state]
struct StateA;

#[group(G)]
impl Testing for (StateA,) {
    type Meta = u8;
}

fn main() {}
