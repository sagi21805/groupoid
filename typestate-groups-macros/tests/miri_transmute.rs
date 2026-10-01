// `transmute_state` between a `u8` state and a `bool` state, meant to run
// under `cargo +nightly miri test --test miri_transmute`.
use typestate_groups::Isomorphic;
use typestate_groups_macros::{group, state, state_types, typestate};

#[state_types]
trait Meta {
    type Value;
}

#[state]
struct Raw;
#[state]
struct Flag;

#[group(RawGroup)]
impl Meta for (Raw,) {
    #[size(1)]
    type Value = u8;
}

#[group(FlagGroup)]
impl Meta for (Flag,) {
    #[size(1)]
    type Value = bool;
}

#[typestate(state = S, unsafe_transmute = true)]
struct Cell<S: Meta> {
    value: S::Value,
}

#[test]
fn transmute_u8_one_into_bool_true() {
    let raw = Cell::<Raw> { value: 1 };
    // SAFETY: 1 is a valid bit pattern for `bool`.
    let flag: Cell<Flag> = unsafe { raw.transmute_state() };
    assert!(flag.value);
}
