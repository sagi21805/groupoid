// `cast_state` rejects a `u8` field that would become a `bool`.
#![allow(dead_code)]
use typestate_groups::Isomorphic;
use typestate_groups_macros::{group, state, state_types, typestate};

#[state_types]
trait Meta {
    type Value;
}

#[state]
struct A;
#[state]
struct B;

#[group(AGroup)]
impl Meta for (A,) {
    #[size(1)]
    type Value = u8;
}

#[group(BGroup)]
impl Meta for (B,) {
    #[size(1)]
    type Value = bool;
}

#[typestate(unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Value,
}

fn cast(a: Wrap<A>) {
    let _ = a.cast_state::<B>();
}

fn main() {}
