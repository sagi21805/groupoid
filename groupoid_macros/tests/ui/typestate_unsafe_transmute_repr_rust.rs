// `unsafe_transmute = true` rejects a `repr` without a guaranteed
// layout.
#![allow(dead_code)]
use groupoid_macros::{group, state, template, typestate};

#[template]
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

#[typestate(state = S, unsafe_transmute = true)]
#[repr(Rust)]
struct Wrap<S: Meta> {
    value: S::Value,
    tag: u8,
}

fn main() {}
