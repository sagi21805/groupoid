// `morph` needs a `MorphFrom` impl from the source state to the target.
#![allow(dead_code)]
use typestate_groups::Morphic;
use typestate_groups_macros::{group, state, state_types, typestate};

#[state_types]
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
