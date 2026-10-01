// A `S::Marker` field has no layout and nothing to convert.
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
