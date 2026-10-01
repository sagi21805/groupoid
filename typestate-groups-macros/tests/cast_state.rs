//! `cast_state` and its ref/mut forms between states whose fields stay
//! valid.

use core::{cell::Cell, marker::PhantomData};
use typestate_groups::Isomorphic;
use typestate_groups_macros::{group, state, state_types, typestate};
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[state_types]
trait Meta {
    type Value;
    type Extra;
}

#[state]
struct Unsigned;
#[state]
struct Signed;
#[state]
struct Pixel;

#[derive(FromBytes, IntoBytes, Immutable)]
#[repr(C)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[group(UnsignedGroup)]
impl Meta for (Unsigned,) {
    #[size(4)]
    type Value = u32;
    #[size(2)]
    type Extra = [u8; 2];
}

#[group(SignedGroup)]
impl Meta for (Signed,) {
    #[size(4)]
    type Value = i32;
    #[size(2)]
    type Extra = u16;
}

#[group(PixelGroup)]
impl Meta for (Pixel,) {
    #[size(4)]
    type Value = Rgba;
    #[size(2)]
    type Extra = i16;
}

#[typestate(state = S, unsafe_transmute = true, align = 8)]
struct Wrap<S: Meta, T> {
    value: S::Value,
    extra: S::Extra,
    payload: T,
    _state: PhantomData<S>,
}

#[state_types]
trait Flagged {
    type Value;
}

#[state]
struct Flag;
#[state]
struct Byte;
#[state]
struct Shared;

#[group(FlagGroup)]
impl Flagged for (Flag,) {
    #[size(1)]
    type Value = bool;
}

#[group(ByteGroup)]
impl Flagged for (Byte,) {
    #[size(1)]
    type Value = u8;
}

#[group(SharedGroup)]
impl Flagged for (Shared,) {
    #[size(1)]
    type Value = Cell<u8>;
}

#[typestate(unsafe_transmute = true)]
struct Slot<S: Flagged> {
    value: S::Value,
}

#[test]
fn cast_state_reinterprets_every_projection() {
    let unsigned = Wrap::<Unsigned, u64> {
        value: u32::MAX,
        extra: [0x34, 0x12],
        payload: 7,
        _state: PhantomData,
    };

    let signed: Wrap<Signed, u64> = unsigned.cast_state();
    assert_eq!(signed.value, -1);
    assert_eq!(signed.extra, u16::from_ne_bytes([0x34, 0x12]));
    assert_eq!(signed.payload, 7);
}

#[test]
fn cast_state_reads_a_derived_struct_as_an_integer() {
    let pixel = Wrap::<Pixel, ()> {
        value: Rgba {
            r: 1,
            g: 2,
            b: 3,
            a: 4,
        },
        extra: -1,
        payload: (),
        _state: PhantomData,
    };

    let unsigned = pixel.cast_state::<Unsigned>();
    assert_eq!(unsigned.value, u32::from_ne_bytes([1, 2, 3, 4]));
    assert_eq!(unsigned.extra, [0xff, 0xff]);
}

#[test]
fn cast_state_ref_and_mut_round_trip() {
    let mut signed = Wrap::<Signed, u8> {
        value: -2,
        extra: 0,
        payload: 0,
        _state: PhantomData,
    };

    assert_eq!(signed.cast_state_ref::<Unsigned>().value, u32::MAX - 1);
    signed.cast_state_mut::<Unsigned>().value = 5;
    assert_eq!(signed.value, 5);
}

#[test]
fn cast_state_turns_a_bool_into_a_byte() {
    let flag = Slot::<Flag> { value: true };

    assert_eq!(flag.cast_state_ref::<Byte>().value, 1);
    assert_eq!(flag.cast_state::<Byte>().value, 1);
}

#[test]
fn cast_state_mut_and_by_value_accept_cells() {
    let mut shared = Slot::<Shared> {
        value: Cell::new(3),
    };

    shared.cast_state_mut::<Byte>().value = 4;
    assert_eq!(shared.value.get(), 4);
    assert_eq!(shared.cast_state::<Byte>().value, 4);
}
