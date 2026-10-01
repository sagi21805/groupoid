// Documents that stacking more than one `#[size(N)]` on the same associated
// type inside a #[group] impl is rejected at macro-expansion time, rather
// than silently picking one of two conflicting layouts.
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
    #[size(8)]
    type Meta = u64;
}

fn main() {}
