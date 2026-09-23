// `TransmuteState::transmute_state` (and friends) take only the target state
// and land in `TransmutableState::Target`, which `#[typestate]` always sets
// to the *same* struct with the state swapped. So two same-sized but
// otherwise unrelated `WithState` types are not merely rejected by a trait
// bound - there is no way to even ask for the cast. `WrapA<A>` into state `A`
// is `WrapA<A>`, and handing that back as a `WrapB<A>` is an ordinary
// mismatched-types error.
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

#[typestate(state = S, unsafe_transmute = true)]
struct WrapA<S: Meta> {
    value: S::Value,
}

#[typestate(state = S, unsafe_transmute = true)]
struct WrapB<S: Meta> {
    value: S::Value,
}

fn cast(w: WrapA<A>) -> WrapB<A> {
    unsafe { w.transmute_state::<A>() }
}

fn main() {
    let w = WrapA { value: 1u32 };
    let _ = cast(w);
}
