// `align = N` below a state's own alignment leaves the container
// 8-aligned as `Wrap<Wide>` and 2-aligned as `Wrap<Bytes>`.
//
// `transmute_state` evaluates `LAYOUT_CHECK` only under `cargo build`, and
// trybuild runs `cargo check`, so a const item forces it here.
#![allow(dead_code)]
use groupoid::TransmutableState;
use groupoid_macros::{group, state, template, typestate};

#[template]
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

#[typestate(state = S, unsafe_transmute = true, align = 2)]
struct Wrap<S: Meta> {
    value: S::Value,
}

const _: () = <Wrap<Wide> as TransmutableState<Bytes>>::LAYOUT_CHECK;

fn main() {}
