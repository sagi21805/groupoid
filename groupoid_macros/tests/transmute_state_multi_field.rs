//! Structs with several fields transmute between states when every field
//! is a bare projection, a ZST, or independent of the state.

use core::marker::PhantomData;
use groupoid::Isomorphic;
use groupoid_macros::{group, state, template, typestate};

#[template]
trait Meta {
    type Value;
}

// --- states whose values agree on size *and* alignment
// ----------------------

#[state]
struct Small;
#[state]
struct Big;

#[group(SmallGroup)]
impl Meta for (Small,) {
    #[size(4)]
    type Value = u32;
}

#[group(BigGroup)]
impl Meta for (Big,) {
    #[size(4)]
    type Value = i32;
}

#[typestate(state = S, unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Value,
    tag: u8,
    _state: PhantomData<S>,
}

#[test]
fn multi_field_struct_transmutes_between_states() {
    assert_eq!(
        size_of::<Wrap<Small>>(),
        size_of::<Wrap<Big>>(),
        "pinning size and alignment should make the two layouts identical"
    );
    assert_eq!(align_of::<Wrap<Small>>(), align_of::<Wrap<Big>>());

    let small = Wrap::<Small> {
        value: 0xdead_beef,
        tag: 7,
        _state: PhantomData,
    };
    let big = unsafe { small.transmute_state::<Big>() };

    assert_eq!(big.value, 0xdead_beefu32 as i32);
    assert_eq!(
        big.tag, 7,
        "the state-independent field survives the transmute"
    );
}

// --- the projection need not come first, and other type params ride along
// ---

#[typestate(state = S, unsafe_transmute = true)]
struct Ordered<S: Meta, T> {
    head: u16,
    value: S::Value,
    tail: T,
}

// Only the state is named at the call; `T = u64` is carried over into the
// target by `TransmutableState::Target`, which is why the annotation on
// `big` is a check rather than a hint.
#[test]
fn projection_need_not_be_the_first_field_and_other_generics_carry_over() {
    let small = Ordered::<Small, u64> {
        head: 1,
        value: 2,
        tail: 3,
    };
    let big: Ordered<Big, u64> = unsafe { small.transmute_state::<Big>() };

    assert_eq!((big.head, big.value, big.tail), (1, 2, 3));
}

// --- every syntactically recognizable ZST is ignored, not just
// `PhantomData` ---

#[typestate(state = S, unsafe_transmute = true)]
struct Zsts<S: Meta> {
    value: S::Value,
    _unit: (),
    _pinned: core::marker::PhantomPinned,
    _state: ::std::marker::PhantomData<fn() -> S>,
}

#[test]
fn unit_and_marker_zsts_do_not_disqualify_the_struct() {
    let small = Zsts::<Small> {
        value: 5,
        _unit: (),
        _pinned: core::marker::PhantomPinned,
        _state: PhantomData,
    };
    let big: Zsts<Big> = unsafe { small.transmute_state::<Big>() };

    assert_eq!(big.value, 5);
}

// --- forced alignment bridges states that disagree on alignment
// -------------

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

// `u64` and `[u8; 8]` differ in alignment, so `align = 8` pins it.
#[typestate(state = S, unsafe_transmute = true, align = 8)]
struct Forced<S: Meta> {
    value: S::Value,
    tag: u8,
}

#[test]
fn forced_alignment_bridges_differently_aligned_states() {
    assert_eq!(align_of::<Forced<Wide>>(), 8);
    assert_eq!(
        align_of::<Forced<Bytes>>(),
        8,
        "the forced repr(align(8)) applies even though [u8; 8] is \
         1-aligned"
    );
    assert_eq!(size_of::<Forced<Wide>>(), size_of::<Forced<Bytes>>());

    let wide = Forced::<Wide> {
        value: u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
        tag: 9,
    };
    let bytes = unsafe { wide.transmute_state::<Bytes>() };

    assert_eq!(bytes.value, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(bytes.tag, 9);
}
