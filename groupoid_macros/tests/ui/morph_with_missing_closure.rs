// `TripleMorph` needs one closure per projection `Triple` uses.
#![allow(dead_code)]
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
    type Type1 = f32;
    type Type2 = i64;
    type Type3 = i16;
}

#[typestate(state = S)]
struct Triple<S: Meta> {
    a: S::Type1,
    b: S::Type2,
    c: S::Type3,
}

fn main() {
    let a = Triple::<StateA> { a: 1, b: 2, c: 3 };
    let _: Triple<StateB> = a.morph_with(TripleMorph {
        type1: &mut |x| x as f32,
        type2: &mut |x| x as i64,
    });
}
