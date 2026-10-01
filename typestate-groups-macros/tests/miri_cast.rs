//! Safe casts that Miri must find free of undefined behaviour, run with
//! `cargo +nightly miri test --test miri_cast`.

use core::marker::PhantomData;
use typestate_groups::Isomorphic;
use typestate_groups_macros::{group, state, state_types, typestate};
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[state_types]
trait Meta {
    type Value;
}

#[state]
struct Unsigned;
#[state]
struct Signed;
#[state]
struct Pixel;

#[derive(FromBytes, IntoBytes, Immutable, Debug, PartialEq)]
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
}

#[group(SignedGroup)]
impl Meta for (Signed,) {
    #[size(4)]
    type Value = i32;
}

#[group(PixelGroup)]
impl Meta for (Pixel,) {
    #[size(4)]
    type Value = Rgba;
}

/// `tag` leaves padding bytes before `payload`.
#[typestate(state = S, unsafe_transmute = true, align = 8)]
struct Wrap<S: Meta, T> {
    value: S::Value,
    tag: u8,
    payload: T,
    _state: PhantomData<S>,
}

fn wrap<S: Meta, T>(value: S::Value, payload: T) -> Wrap<S, T> {
    Wrap {
        value,
        tag: 9,
        payload,
        _state: PhantomData,
    }
}

#[test]
fn cast_state_moves_a_heap_payload_without_double_drop_or_leak() {
    let unsigned = wrap::<Unsigned, _>(u32::MAX, String::from("owned"));

    let signed = unsigned.cast_state::<Signed>();
    assert_eq!(signed.value, -1);
    assert_eq!(signed.payload, "owned");
}

#[test]
fn cast_state_copies_over_container_padding() {
    let pixel = wrap::<Pixel, _>(
        Rgba {
            r: 1,
            g: 2,
            b: 3,
            a: 4,
        },
        Box::new(5u64),
    );

    let unsigned = pixel.cast_state::<Unsigned>();
    assert_eq!(unsigned.tag, 9);
    assert_eq!(unsigned.value, u32::from_ne_bytes([1, 2, 3, 4]));
    assert_eq!(*unsigned.payload, 5);
}

#[test]
fn cast_state_ref_interleaves_with_other_shared_borrows() {
    let signed = wrap::<Signed, _>(-2, vec![1u8, 2]);
    let plain = &signed;
    let unsigned = signed.cast_state_ref::<Unsigned>();
    let pixel = signed.cast_state_ref::<Pixel>();

    assert_eq!(unsigned.value, u32::MAX - 1);
    assert_eq!(plain.value, -2);
    assert_eq!(pixel.value.r, 0xfe);
    assert_eq!(unsigned.payload, plain.payload);
}

#[test]
fn cast_state_mut_writes_are_seen_after_the_borrow_ends() {
    let mut unsigned = wrap::<Unsigned, _>(0, String::new());

    {
        let pixel = unsigned.cast_state_mut::<Pixel>();
        pixel.value.a = 0x80;
        pixel.payload.push_str("written");
    }
    assert_eq!(unsigned.value, u32::from_ne_bytes([0, 0, 0, 0x80]));

    unsigned.cast_state_mut::<Signed>().value = i32::MIN;
    assert_eq!(unsigned.value, 1 << 31);
    assert_eq!(unsigned.payload, "written");
}

#[test]
fn cast_state_mut_nests_through_several_states() {
    let mut unsigned = wrap::<Unsigned, ()>(7, ());

    let signed = unsigned.cast_state_mut::<Signed>();
    let pixel = signed.cast_state_mut::<Pixel>();
    pixel.value.r = 8;

    assert_eq!(unsigned.value.to_ne_bytes()[0], 8);
}
