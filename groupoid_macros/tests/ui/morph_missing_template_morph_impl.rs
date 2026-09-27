// `morph` needs the morpher to implement `MetaMorph` for these two states.
#![allow(dead_code)]
use groupoid_macros::{group, state, template, typestate};

#[template]
trait Meta {
    type Value;
}

#[state]
struct StateA;
#[state]
struct StateB;
#[state]
struct StateC;

#[group(AGroup)]
impl Meta for (StateA,) {
    type Value = u32;
}

#[group(BGroup)]
impl Meta for (StateB,) {
    type Value = u64;
}

#[group(CGroup)]
impl Meta for (StateC,) {
    type Value = u8;
}

#[typestate(state = S)]
struct Wrap<S: Meta> {
    value: S::Value,
}

struct Widen;

impl MetaMorph<StateA, StateB> for Widen {
    fn value(&mut self, value: u32) -> u64 {
        value.into()
    }
}

fn main() {
    let a = Wrap::<StateA> { value: 1 };
    let _ = a.morph::<StateC, _>(Widen);
}
