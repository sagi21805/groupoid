// `unsafe_transmute = true` on a struct that never projects through its state
// has nothing to transmute: `TransmutableState` would relate two identical
// layouts for no reason, so the flag is rejected. Plain `#[typestate]` accepts
// such a struct - it is a valid `WithState` container, it just has no state
// transition to generate.
#![allow(dead_code)]
use core::marker::PhantomData;
use groupoid_macros::{state, typestate};

#[state]
struct Small;

#[typestate(state = S, unsafe_transmute = true)]
struct Tagged<S> {
    tag: u8,
    _s: PhantomData<S>,
}

fn main() {}
