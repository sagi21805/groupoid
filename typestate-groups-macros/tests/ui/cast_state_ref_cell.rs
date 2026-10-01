// `cast_state_ref` rejects a field that holds a `Cell`.
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
    #[size(4)]
    type Value = core::cell::Cell<u32>;
}

#[group(BGroup)]
impl Meta for (B,) {
    #[size(4)]
    type Value = u32;
}

#[typestate(unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Value,
}

fn cast(a: Wrap<A>) {
    let _ = a.cast_state_ref::<B>();
}

fn main() {}
