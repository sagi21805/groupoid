// A generic `MorphFrom` impl that reuses a field's conversion reports the
// field's missing impl.
#![allow(dead_code)]
use typestate_groups::{MorphFrom, Morphic};
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
struct Inner<S: Meta> {
    value: S::Value,
}

#[typestate(state = S)]
struct Outer<S: Meta> {
    inner: Inner<S>,
}

impl<S: Meta, S2: Meta> MorphFrom<Outer<S>> for Outer<S2>
where
    Inner<S2>: MorphFrom<Inner<S>>,
{
    fn morph_from(src: Outer<S>) -> Self {
        Outer {
            inner: src.inner.morph::<S2>(),
        }
    }
}

fn main() {
    let _ = Outer::<StateA> {
        inner: Inner { value: 1 },
    }
    .morph::<StateB>();
}
