// `align` requires `unsafe_transmute = true`.
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
