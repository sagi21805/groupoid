// `align` requires `unsafe_transmute = true`.
#![allow(dead_code)]
use typestate_groups_macros::{group, state, state_types, typestate};

#[state_types]
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
