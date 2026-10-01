//! Unchecked `transmute_state` calls that Miri must reject as undefined
//! behaviour. `cast_state` refuses each of them at compile time; the
//! matching UI test is named on every case.
//!
//! UB aborts the test binary instead of panicking, so every case is
//! ignored and `miri-expect-ub.sh` runs each one alone, requiring Miri to
//! report undefined behaviour.

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

/// `cast_state_invalid_target`
#[test]
#[ignore = "undefined behaviour: run with miri-expect-ub.sh"]
fn transmute_state_u8_two_into_bool() {
    let raw = Small::<Raw> { value: 2 };
    let flag: Small<Flag> = unsafe { raw.transmute_state() };
    assert!(flag.value);
}

/// `cast_state_invalid_target`
#[test]
#[ignore = "undefined behaviour: run with miri-expect-ub.sh"]
fn transmute_state_surrogate_into_char() {
    let bits = Large::<Bits> { value: 0xd800 };
    let letter: Large<Letter> = unsafe { bits.transmute_state() };
    assert_eq!(letter.value.len_utf8(), 3);
}

/// `cast_state_invalid_target`
#[test]
#[ignore = "undefined behaviour: run with miri-expect-ub.sh"]
fn transmute_state_zero_into_non_zero() {
    let bits = Large::<Bits> { value: 0 };
    let count: &Large<Count> = unsafe { bits.transmute_state_ref() };
    assert_eq!(count.value.get(), 0);
}

/// `cast_state_padded_source`
#[test]
#[ignore = "undefined behaviour: run with miri-expect-ub.sh"]
fn transmute_state_reads_padding_as_an_integer() {
    let gapped = Large::<Gapped> {
        value: Padded { small: 1, wide: 2 },
    };
    let bits: Large<Bits> = unsafe { gapped.transmute_state() };
    assert_ne!(bits.value, 0);
}

/// `cast_state_mut_one_way`
#[test]
#[ignore = "undefined behaviour: run with miri-expect-ub.sh"]
fn transmute_state_mut_writes_an_invalid_bool_back() {
    let mut flag = Small::<Flag> { value: false };
    unsafe { flag.transmute_state_mut::<Raw>() }.value = 2;
    assert!(flag.value);
}

/// `cast_state_ref_cell`
#[test]
#[ignore = "undefined behaviour: run with miri-expect-ub.sh"]
fn transmute_state_ref_writes_through_a_shared_integer() {
    let bits = Large::<Bits> { value: 1 };
    let shared: &Large<Shared> = unsafe { bits.transmute_state_ref() };
    shared.value.set(2);
    assert_eq!(bits.value, 2);
}
