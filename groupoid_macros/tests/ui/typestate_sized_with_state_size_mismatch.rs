// Documents that `SizedWithState<N>` is only implemented for the `N` the
// state's group actually pinned via `#[size(N)]` - asking for a different
// `N` is an ordinary trait-bound failure, not something that silently
// passes.
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
struct Wrap<S: Meta> {
    value: S::Value,
}

fn assert_sized_with_state<T: groupoid::SizedWithState<N>, const N: usize>() {}

fn main() {
    assert_sized_with_state::<Wrap<Small>, 8>();
}
