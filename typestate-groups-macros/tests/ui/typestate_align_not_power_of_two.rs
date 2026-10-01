// rustc rejects a bad `align = N`, pointing at the caller's literal.
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

#[typestate(state = S, unsafe_transmute = true, align = 3)]
struct NotPowerOfTwo<S: Meta> {
    value: S::Value,
}

#[typestate(state = S, unsafe_transmute = true, align = 1073741824)]
struct TooLarge<S: Meta> {
    value: S::Value,
}

fn main() {}
