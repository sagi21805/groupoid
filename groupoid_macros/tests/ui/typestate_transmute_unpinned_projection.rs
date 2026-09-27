// A projection without `#[size(N)]` in either state doesn't transmute.
#![allow(dead_code)]
use groupoid::Isomorphic;
use groupoid_macros::{group, state, template, typestate};

#[template]
trait Meta {
    type Value;
}

#[state]
struct Pinned;
#[state]
struct Loose;

#[group(PinnedGroup)]
impl Meta for (Pinned,) {
    #[size(4)]
    type Value = u32;
}

#[group(LooseGroup)]
impl Meta for (Loose,) {
    type Value = u32;
}

#[typestate(state = S, unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Value,
}

fn main() {
    let pinned = Wrap::<Pinned> { value: 0 };
    let _ = unsafe { pinned.transmute_state::<Loose>() };

    let loose = Wrap::<Loose> { value: 0 };
    let _ = unsafe { loose.transmute_state::<Pinned>() };
}
