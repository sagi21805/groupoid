// `#[size(N)]` that doesn't match its type, and `transmute_state` between
// states whose layouts differ.
//
// `transmute_state` evaluates `LAYOUT_CHECK` only under `cargo build`, and
// trybuild runs `cargo check`, so const items force it here.
#![allow(dead_code)]
use typestate_groups::{Isomorphic, TransmutableState};
use typestate_groups_macros::{group, state, state_types, typestate};

#[state_types]
trait Meta {
    type Type1;
    type Type2;
}

#[state]
struct Small;
#[state]
struct Big;
#[state]
struct Loose;
#[state]
struct Wide;
#[state]
struct Bytes;
#[state]
struct Mislabeled;

#[group(SmallGroup)]
impl Meta for (Small,) {
    #[size(1)]
    type Type1 = u8;
    #[size(8)]
    type Type2 = u64;
}

#[group(BigGroup)]
impl Meta for (Big,) {
    #[size(8)]
    type Type1 = u64;
    #[size(4)]
    type Type2 = u32;
}

#[group(LooseGroup)]
impl Meta for (Loose,) {
    type Type1 = u8;
    #[size(8)]
    type Type2 = u64;
}

#[group(WideGroup)]
impl Meta for (Wide,) {
    #[size(8)]
    type Type1 = u64;
    #[size(8)]
    type Type2 = u64;
}

#[group(BytesGroup)]
impl Meta for (Bytes,) {
    #[size(8)]
    type Type1 = [u8; 8];
    #[size(8)]
    type Type2 = [u8; 8];
}

#[group(MislabeledGroup)]
impl Meta for (Mislabeled,) {
    #[size(4)]
    type Type1 = u64;
    #[size(8)]
    type Type2 = u64;
}

#[typestate(state = S, unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Type1,
}

#[typestate(state = S, unsafe_transmute = true)]
struct Pair<S: Meta> {
    first: S::Type1,
    second: S::Type2,
}

#[typestate(state = S, unsafe_transmute = true)]
struct Other<S: Meta> {
    value: S::Type1,
}

#[typestate(state = S, unsafe_transmute = true, align = 8)]
struct Offset<S: Meta> {
    tag: u8,
    value: S::Type1,
}

#[typestate(state = S, unsafe_transmute = true, align = 2)]
struct TooSmall<S: Meta> {
    value: S::Type1,
}

fn size_mismatch(small: Wrap<Small>) {
    let _ = unsafe { small.transmute_state::<Big>() };
}

// Only `second` differs, and the error names it.
fn one_field_size_mismatch(big: Pair<Big>) {
    let _ = unsafe { big.transmute_state::<Wide>() };
}

fn unpinned_projection(small: Wrap<Small>, loose: Wrap<Loose>) {
    let _ = unsafe { small.transmute_state::<Loose>() };
    let _ = unsafe { loose.transmute_state::<Small>() };
}

fn align_disagreement(wide: Wrap<Wide>) {
    let _ = unsafe { wide.transmute_state::<Bytes>() };
}

fn unrelated_structs(wrap: Wrap<Small>) -> Other<Small> {
    unsafe { wrap.transmute_state::<Small>() }
}

// `align = N` pins the container's alignment, not its fields': `value`
// sits at offset 8 as a `u64` and at offset 1 as a `[u8; 8]`.
const _: () = <Offset<Wide> as TransmutableState<Bytes>>::LAYOUT_CHECK;

// `align = 2` leaves `TooSmall<Wide>` 8-aligned.
const _: () = <TooSmall<Wide> as TransmutableState<Bytes>>::LAYOUT_CHECK;

fn main() {}
