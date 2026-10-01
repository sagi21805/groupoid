// The cases of `tests/miri_ub.rs`, where Miri finds undefined behaviour
// in `transmute_state`, written with `cast_state`: none of them compile.
#![allow(dead_code)]

use core::{cell::Cell, num::NonZeroU32};
use typestate_groups::Isomorphic;
use typestate_groups_macros::{group, state, state_types, typestate};

#[state_types]
trait Byte {
    type Value;
}

#[state]
struct Raw;
#[state]
struct Flag;

#[group(RawGroup)]
impl Byte for (Raw,) {
    #[size(1)]
    type Value = u8;
}

#[group(FlagGroup)]
impl Byte for (Flag,) {
    #[size(1)]
    type Value = bool;
}

#[typestate(unsafe_transmute = true)]
struct Small<S: Byte> {
    value: S::Value,
}

#[state_types]
trait Word {
    type Value;
}

#[state]
struct Bits;
#[state]
struct Letter;
#[state]
struct Count;
#[state]
struct Shared;
#[state]
struct Gapped;

/// One padding byte after `small`.
#[repr(C)]
struct Padded {
    small: u8,
    wide: u16,
}

#[group(BitsGroup)]
impl Word for (Bits,) {
    #[size(4)]
    type Value = u32;
}

#[group(LetterGroup)]
impl Word for (Letter,) {
    #[size(4)]
    type Value = char;
}

#[group(CountGroup)]
impl Word for (Count,) {
    #[size(4)]
    type Value = NonZeroU32;
}

#[group(SharedGroup)]
impl Word for (Shared,) {
    #[size(4)]
    type Value = Cell<u32>;
}

#[group(GappedGroup)]
impl Word for (Gapped,) {
    #[size(4)]
    type Value = Padded;
}

#[typestate(unsafe_transmute = true, align = 4)]
struct Large<S: Word> {
    value: S::Value,
}

fn transmute_state_u8_two_into_bool(raw: Small<Raw>) {
    let _ = raw.cast_state::<Flag>();
}

fn transmute_state_surrogate_into_char(bits: Large<Bits>) {
    let _ = bits.cast_state::<Letter>();
}

fn transmute_state_zero_into_non_zero(bits: &Large<Bits>) {
    let _ = bits.cast_state_ref::<Count>();
}

fn transmute_state_reads_padding_as_an_integer(gapped: Large<Gapped>) {
    let _ = gapped.cast_state::<Bits>();
}

fn transmute_state_mut_writes_an_invalid_bool_back(flag: &mut Small<Flag>) {
    flag.cast_state_mut::<Raw>().value = 2;
}

fn transmute_state_ref_writes_through_a_shared_integer(bits: &Large<Bits>) {
    bits.cast_state_ref::<Shared>().value.set(2);
}

fn main() {}
