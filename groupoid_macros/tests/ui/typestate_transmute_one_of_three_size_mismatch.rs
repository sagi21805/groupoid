// One projection out of three differs in size, and the error names it.
#![allow(dead_code)]
use groupoid::Isomorphic;
use groupoid_macros::{group, state, template, typestate};

#[template]
trait Meta {
    type Type1;
    type Type2;
    type Type3;
}

#[state]
struct StateA;
#[state]
struct StateB;

#[group(AGroup)]
impl Meta for (StateA,) {
    #[size(4)]
    type Type1 = u32;
    #[size(8)]
    type Type2 = u64;
    #[size(2)]
    type Type3 = u16;
}

#[group(BGroup)]
impl Meta for (StateB,) {
    #[size(4)]
    type Type1 = f32;
    #[size(4)]
    type Type2 = u32;
    #[size(2)]
    type Type3 = i16;
}

#[typestate(state = S, unsafe_transmute = true)]
struct Triple<S: Meta> {
    a: S::Type1,
    b: S::Type2,
    c: S::Type3,
}

fn main() {
    let a = Triple::<StateA> { a: 1, b: 2, c: 3 };
    let _ = unsafe { a.transmute_state::<StateB>() };
}
