// `restate_with` takes one leaf conversion, typed by the associated type the
// struct projects through its state. A struct projecting two distinct ones
// (`S::Value` and `S::Marker` - the latter is the associated type `#[blueprint]`
// adds) would leave that leaf ambiguous, so `#[typestate]` rejects it outright.
#![allow(dead_code)]
use groupoid_macros::{blueprint, typestate};

#[blueprint]
trait Meta {
    type Value;
}

#[typestate(state = S)]
struct Wrap<S: Meta> {
    value: S::Value,
    marker: S::Marker,
}

fn main() {}
