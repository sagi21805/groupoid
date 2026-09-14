// Documents that `#[size(N)]` on an associated type inside a #[group] impl
// generates a `const _: () = assert!(size_of::<T>() == N, ..)`, which fails
// to compile (rather than silently passing) when the concrete type's actual
// size doesn't match the declared one.
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
