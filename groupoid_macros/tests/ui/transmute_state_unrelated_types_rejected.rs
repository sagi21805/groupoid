// `TransmuteState::transmute_state` (and friends) require
// `Self: TransmutableState<To, _>`, which `#[typestate]` only ever generates
// between two states of the *same* struct - two same-sized but otherwise
// unrelated `WithState` types must not be castable into each other just
// because `size_of` happens to match.
#![allow(dead_code)]
use groupoid::TransmuteState;
use groupoid_macros::{blueprint, group, state, typestate};

#[blueprint]
trait Meta {
    type Value;
}

#[state]
struct A;
#[state]
struct B;

#[group(AGroup)]
impl Meta for (A,) {
    #[size(4)]
    type Value = u32;
}

#[group(BGroup)]
impl Meta for (B,) {
    #[size(4)]
    type Value = u32;
}

#[typestate(state = S)]
struct WrapA<S: Meta> {
    value: S::Value,
}

#[typestate(state = S)]
struct WrapB<S: Meta> {
    value: S::Value,
}

fn cast(w: WrapA<A>) -> WrapB<A> {
    unsafe { w.transmute_state() }
}

fn main() {
    let w = WrapA { value: 1u32 };
    let _ = cast(w);
}
