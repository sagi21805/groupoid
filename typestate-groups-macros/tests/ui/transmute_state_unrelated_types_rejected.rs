// `transmute_state` can't turn one struct into another, even at the same
// size.
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
    type Value = u32;
}

#[group(BGroup)]
impl Meta for (B,) {
    #[size(4)]
    type Value = u32;
}

#[typestate(state = S, unsafe_transmute = true)]
struct WrapA<S: Meta> {
    value: S::Value,
}

#[typestate(state = S, unsafe_transmute = true)]
struct WrapB<S: Meta> {
    value: S::Value,
}

fn cast(w: WrapA<A>) -> WrapB<A> {
    unsafe { w.transmute_state::<A>() }
}

fn main() {
    let w = WrapA { value: 1u32 };
    let _ = cast(w);
}
