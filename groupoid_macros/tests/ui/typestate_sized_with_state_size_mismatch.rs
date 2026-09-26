// `SizedWithState<N>` exists only for the group's `#[size(N)]`.
#![allow(dead_code)]
use groupoid_macros::{group, state, template, typestate};

#[template]
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

#[typestate(state = S, unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Value,
}

fn assert_sized_with_state<T: groupoid::SizedWithState<N, A>, const N: usize, const A: usize>() {}

fn main() {
    // `u8` pins the group at size 1, alignment 1; asking for size 8 fails.
    assert_sized_with_state::<Wrap<Small>, 8, 1>();
}
