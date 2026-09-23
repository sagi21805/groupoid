// `unsafe_transmute = true` reasons about the struct's field offsets, which
// only `repr(C)` (or `repr(transparent)`) guarantees: under `repr(Rust)` the
// compiler may order the fields of `Wrap<Small>` and `Wrap<Big>` differently,
// because the field *types* differ. An explicit `repr` from the author is
// respected, so one that gives no such guarantee is rejected rather than
// silently overridden.
//
// Without the flag the same attribute is fine - see
// `restate_only_structs_keep_their_own_repr` in `tests/restate.rs`.
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

#[typestate(state = S, unsafe_transmute = true)]
#[repr(Rust)]
struct Wrap<S: Meta> {
    value: S::Value,
    tag: u8,
}

fn main() {}
