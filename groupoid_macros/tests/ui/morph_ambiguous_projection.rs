// Projecting two associated types makes `morph_with`'s `f` ambiguous.
#![allow(dead_code)]
use groupoid_macros::{template, typestate};

#[template]
trait Meta {
    type Value;
}

#[typestate(state = S)]
struct Wrap<S: Meta> {
    value: S::Value,
    marker: S::Marker,
}

fn main() {}
