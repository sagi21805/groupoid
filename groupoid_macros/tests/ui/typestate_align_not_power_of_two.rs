// rustc rejects a bad `align = N`, pointing at the caller's literal.
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

#[typestate(state = S, unsafe_transmute = true, align = 3)]
struct NotPowerOfTwo<S: Meta> {
    value: S::Value,
}

#[typestate(state = S, unsafe_transmute = true, align = 1073741824)]
struct TooLarge<S: Meta> {
    value: S::Value,
}

fn main() {}
