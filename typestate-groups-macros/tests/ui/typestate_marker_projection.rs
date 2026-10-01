// A `S::Marker` field has no layout and nothing to convert.
#![allow(dead_code)]
use typestate_groups_macros::{state_types, typestate};

#[state_types]
trait Meta {
    type Value;
}

#[typestate(state = S)]
struct Wrap<S: Meta> {
    value: S::Value,
    marker: S::Marker,
}

fn main() {}
