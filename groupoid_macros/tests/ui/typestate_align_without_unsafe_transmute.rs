// `align` pins the container's alignment for the in-place `transmute_state`
// path, which only exists under `unsafe_transmute = true`. The by-value
// `restate_with` path never looks at alignment, so an `align` on its own would
// pin nothing - it is rejected rather than silently ignored.
#![allow(dead_code)]
use groupoid_macros::{blueprint, group, state, typestate};

#[blueprint]
trait Meta {
    type Value;
}

#[state]
struct Small;

#[group(SmallGroup)]
impl Meta for (Small,) {
    #[size(1)]
    type Value = u8;
}

#[typestate(state = S, align = 8)]
struct Wrap<S: Meta> {
    value: S::Value,
}

#[typestate(state = S, unsafe_transmute = false, align = 8)]
struct Explicit<S: Meta> {
    value: S::Value,
}

fn main() {}
