//! Casts between states whose fields have invalid bit patterns, meant to
//! run under `cargo +nightly miri test` too.

use core::num::NonZeroU32;
use typestate_groups::Isomorphic;
use typestate_groups_macros::{group, state, state_types, typestate};

#[state_types]
trait Meta {
    type Value;
}

#[state]
struct Raw;
#[state]
struct Letter;
#[state]
struct Count;

#[group(RawGroup)]
impl Meta for (Raw,) {
    #[size(4)]
    type Value = u32;
}

#[group(LetterGroup)]
impl Meta for (Letter,) {
    #[size(4)]
    type Value = char;
}

#[group(CountGroup)]
impl Meta for (Count,) {
    #[size(4)]
    type Value = NonZeroU32;
}

#[typestate(unsafe_transmute = true)]
struct Slot<S: Meta> {
    value: S::Value,
}

#[test]
fn char_and_non_zero_cast_safely_into_integers() {
    let letter = Slot::<Letter> { value: 'z' };
    assert_eq!(letter.cast_state::<Raw>().value, 'z' as u32);

    let count = Slot::<Count> {
        value: NonZeroU32::MIN,
    };
    assert_eq!(count.cast_state_ref::<Raw>().value, 1);
}

#[test]
fn integers_transmute_into_char_and_non_zero_when_valid() {
    let raw = Slot::<Raw> { value: 'z' as u32 };
    // SAFETY: `'z' as u32` is a valid `char`.
    let letter: Slot<Letter> = unsafe { raw.transmute_state() };
    assert_eq!(letter.value, 'z');

    let mut raw = Slot::<Raw> { value: 7 };
    // SAFETY: 7 is a valid `NonZeroU32`, and the borrow writes none.
    let count: &mut Slot<Count> = unsafe { raw.transmute_state_mut() };
    assert_eq!(count.value.get(), 7);
}
