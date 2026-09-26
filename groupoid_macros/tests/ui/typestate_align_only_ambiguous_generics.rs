// `align` on its own still leaves the state parameter to inference, so a struct
// with more than one generic type parameter gets the usual "needs
// `state = <Ident>`" error rather than anything about alignment.
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

#[typestate(unsafe_transmute = true, align = 8)]
struct Ambiguous<S: Meta, T> {
    value: S::Value,
    other: T,
}

fn main() {}
