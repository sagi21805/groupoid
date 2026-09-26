// `unsafe_transmute = true` needs a field that projects through the
// state.
#![allow(dead_code)]
use groupoid_macros::{state, typestate};

#[state]
struct Small;

#[typestate(state = S, unsafe_transmute = true)]
struct Tagged<S> {
    tag: u8,
    _s: core::marker::PhantomData<S>,
}

fn main() {}
