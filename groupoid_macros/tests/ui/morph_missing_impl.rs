// `morph` needs a `MorphFrom` impl from the source state to the target.
#![allow(dead_code)]
use groupoid::Morphic;
use groupoid_macros::{group, state, template, typestate};

#[template]
trait Meta {
    type Value;
}

#[state]
struct StateA;
#[state]
struct StateB;

#[group(AGroup)]
impl Meta for (StateA,) {
    type Value = u32;
}

#[group(BGroup)]
impl Meta for (StateB,) {
    type Value = u64;
}

#[typestate(state = S)]
struct Wrap<S: Meta> {
    value: S::Value,
}

fn main() {
    let _ = Wrap::<StateA> { value: 1 }.morph::<StateB>();
}
