// `cast_state_mut` rejects a `bool` field that would accept any `u8` written through the borrow.
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
    type Value = bool;
}

#[group(BGroup)]
impl Meta for (B,) {
    #[size(1)]
    type Value = u8;
}

#[typestate(unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Value,
}

fn cast(mut a: Wrap<A>) {
    let _ = a.cast_state_mut::<B>();
}

fn main() {}
