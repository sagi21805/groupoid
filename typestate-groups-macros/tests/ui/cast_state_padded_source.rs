// `cast_state` rejects a source field with padding bytes.
#![allow(dead_code)]
use typestate_groups::Isomorphic;
use typestate_groups_macros::{group, state, state_types, typestate};

#[derive(zerocopy::FromBytes)]
#[repr(C)]
struct Padded {
    small: u8,
    wide: u16,
}

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
    type Value = Padded;
}

#[group(BGroup)]
impl Meta for (B,) {
    #[size(4)]
    type Value = [u8; 4];
}

#[typestate(unsafe_transmute = true, align = 4)]
struct Wrap<S: Meta> {
    value: S::Value,
}

fn cast(a: Wrap<A>) {
    let _ = a.cast_state::<B>();
}

fn main() {}
