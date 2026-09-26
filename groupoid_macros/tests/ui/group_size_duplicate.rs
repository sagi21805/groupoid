// Documents that stacking more than one `#[size(N)]` on the same associated
// type inside a #[group] impl is rejected at macro-expansion time, rather
// than silently emitting two conflicting `SizedGroup<N>` impls.
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
    #[size(8)]
    type Meta = u64;
}

fn main() {}
