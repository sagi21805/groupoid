// Documents that `SizedWithState` derivation only applies to a `#[typestate]`
// struct with exactly one field of the bare `S::Assoc` shape - a second
// field means the derivation doesn't apply at all, so the trait simply
// isn't implemented for the struct (not an error at the `#[typestate]` site,
// but a trait-bound failure wherever `SizedWithState` is then required).
#![allow(dead_code)]
use groupoid_macros::{blueprint, group, state, typestate};

#[blueprint]
trait Meta {
    type Value;
}

#[state]
struct Small;

#[group(SmallGroup)]
impl Meta for (Small,) {
    #[size(1)]
    type Value = u8;
}

#[typestate(state = S)]
struct NotSized<S: Meta> {
    value: S::Value,
    extra: u8,
}

fn assert_sized_with_state<T: groupoid::SizedWithState<N>, const N: usize>() {}

fn main() {
    assert_sized_with_state::<NotSized<Small>, 1>();
}
