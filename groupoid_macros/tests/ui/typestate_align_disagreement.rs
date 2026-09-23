// The user-facing face of the alignment feature. `u64` and `[u8; 8]` are both
// 8 bytes, so under a size-only rule these two states would look
// interchangeable - but they are 8- and 1-aligned respectively, so the
// containers holding them do not share a layout, and transmuting between them
// would be undefined behaviour.
//
// The failure is reported against the `AlignedGroup` bound rather than
// `TransmutableState`, which is deliberate: rustc's own
// "but trait `AlignedGroup<1>` is implemented for it" names the alignment the
// other state actually has, so the fix - `#[typestate(state = S, unsafe_transmute = true, align = 8)]`,
// exercised in `tests/transmute_state_multi_field.rs` - is a matter of reading
// off the larger of the two.
#![allow(dead_code)]
use groupoid::TransmuteState;
use groupoid_macros::{blueprint, group, state, typestate};

#[blueprint]
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

#[typestate(state = S, unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Value,
    tag: u8,
}

fn main() {
    let wide = Wrap::<Wide> { value: 0, tag: 0 };
    let _bytes = unsafe { wide.transmute_state::<Bytes>() };
}
