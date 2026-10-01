// `align = N` pins the container's alignment, not its fields': `value`
// sits at offset 8 as a `u64` and at offset 1 as a `[u8; 8]`.
//
// `transmute_state` evaluates `LAYOUT_CHECK` only under `cargo build`, and
// trybuild runs `cargo check`, so a const item forces it here.
#![allow(dead_code)]
use typestate_groups::TransmutableState;
use typestate_groups_macros::{group, state, state_types, typestate};

#[state_types]
trait Meta {
    type Value;
}

#[state]
struct Wide;
#[state]
struct Bytes;

#[group(WideGroup)]
impl Meta for (Wide,) {
    #[size(8)]
    type Value = u64;
}

#[group(BytesGroup)]
impl Meta for (Bytes,) {
    #[size(8)]
    type Value = [u8; 8];
}

#[typestate(state = S, unsafe_transmute = true, align = 8)]
struct Wrap<S: Meta> {
    tag: u8,
    value: S::Value,
}

const _: () = <Wrap<Wide> as TransmutableState<Bytes>>::LAYOUT_CHECK;

fn main() {}
