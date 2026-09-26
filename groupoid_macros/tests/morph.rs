//! `morph_with` rebuilds a struct for another state by value, through
//! `Option`, arrays, `Box`, tuples and user `Morph` impls.

use core::marker::PhantomData;
use groupoid::Morph;
use groupoid_macros::{group, state, template, typestate};
use std::collections::HashMap;

#[template]
trait Meta {
    type Value;
}

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

// --- a user wrapper with its own `Morph` impl ---

#[derive(Debug, PartialEq)]
struct Pair<T>(T, T);

impl<A, B> Morph<A, B> for Pair<A> {
    type Output = Pair<B>;

    fn morph(self, f: &mut impl FnMut(A) -> B) -> Pair<B> {
        Pair(f(self.0), f(self.1))
    }
}

// --- every shape at once
// --------------------------------------------------

#[typestate(state = S)]
struct Wrap<S: Meta> {
    value: S::Value,
    maybe: Option<S::Value>,
    many: [S::Value; 2],
    boxed: Box<S::Value>,
    pair: (S::Value, u8),
    nested: Option<[S::Value; 2]>,
    user: Pair<S::Value>,
    user_nested: Pair<Option<S::Value>>,
    list: Vec<S::Value>,
    result: Result<S::Value, u8>,
    map: HashMap<u8, S::Value>,
    tag: u8,
    _s: PhantomData<S>,
}

fn sample() -> Wrap<Small> {
    Wrap {
        value: 1,
        maybe: Some(2),
        many: [3, 4],
        boxed: Box::new(5),
        pair: (6, 7),
        nested: Some([8, 9]),
        user: Pair(10, 11),
        user_nested: Pair(Some(13), None),
        list: vec![14, 15],
        result: Ok(16),
        map: HashMap::from([(0, 17)]),
        tag: 12,
        _s: PhantomData,
    }
}

fn assert_round_trip(big: Wrap<Big>) {
    assert_eq!(big.value, 1);
    assert_eq!(big.maybe, Some(2));
    assert_eq!(big.many, [3, 4]);
    assert_eq!(*big.boxed, 5);
    assert_eq!(big.pair, (6, 7));
    assert_eq!(big.nested, Some([8, 9]));
    assert_eq!(big.user, Pair(10, 11));
    assert_eq!(big.user_nested, Pair(Some(13), None));
    assert_eq!(big.list, [14, 15]);
    assert_eq!(big.result, Ok(16));
    assert_eq!(big.map, HashMap::from([(0, 17)]));
    assert_eq!(big.tag, 12, "the state-independent field is moved as-is");
}

#[test]
fn morph_with_converts_every_projection_through_f() {
    let mut calls = 0;
    let big: Wrap<Big> = sample().morph_with(|v| {
        calls += 1;
        v as i32
    });
    assert_round_trip(big);
    assert_eq!(
        calls, 15,
        "one call per projection: value, maybe, many x2, boxed, pair.0, \
         nested x2, user x2, user_nested, list x2, result, map"
    );
}

#[test]
fn morph_with_sees_none_without_calling_f() {
    let small = Wrap::<Small> {
        maybe: None,
        nested: None,
        ..sample()
    };
    let big: Wrap<Big> = small.morph_with(|v| v as i32);
    assert_eq!(big.maybe, None);
    assert_eq!(big.nested, None);
}

// --- the `Option` field would be rejected by `unsafe_transmute = true`,
// yet `morph_with` exists ---

#[typestate(state = S)]
struct NotTransmutable<S: Meta> {
    value: Option<S::Value>,
}

#[test]
fn a_struct_without_the_in_place_path_still_morphs_by_value() {
    let big: NotTransmutable<Big> = NotTransmutable::<Small> {
        value: Some(0xdead_beef),
    }
    .morph_with(|v| v as i32);
    assert_eq!(big.value, Some(0xdead_beefu32 as i32));
}

// --- other generics ride along, and tuple structs are positional
// ---------

#[typestate(state = S)]
struct Ordered<S: Meta, T> {
    head: u16,
    value: S::Value,
    tail: T,
}

#[test]
fn other_generics_carry_over_to_the_target() {
    let small = Ordered::<Small, u64> {
        head: 1,
        value: 2,
        tail: 3,
    };
    let big: Ordered<Big, u64> = small.morph_with(|v| v as i32);
    assert_eq!((big.head, big.value, big.tail), (1, 2, 3));
}

#[typestate(state = S)]
struct Positional<S: Meta>(u8, S::Value, Option<S::Value>);

#[test]
fn tuple_structs_are_rebuilt_positionally() {
    let big: Positional<Big> =
        Positional::<Small>(1, 2, Some(3)).morph_with(|v| v as i32);
    assert_eq!((big.0, big.1, big.2), (1, 2, Some(3)));
}

// --- forced alignment still generates `morph_with`
// ---------------------

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
struct Forced<S: Meta> {
    value: S::Value,
    tag: u8,
}

#[test]
fn forced_alignment_mode_still_generates_morph_with() {
    let wide = Forced::<Wide> {
        value: u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
        tag: 9,
    };
    let bytes: Forced<Bytes> = wide.morph_with(u64::to_ne_bytes);
    assert_eq!(bytes.value, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(bytes.tag, 9);
}

// --- with no forced alignment either: `u64` and `[u8; 8]` disagree on
// alignment, which the in-place path rejects but the by-value path never
// asks about ---

#[typestate(state = S)]
struct Unforced<S: Meta> {
    value: S::Value,
    tag: u8,
}

#[test]
fn morph_with_ignores_alignment_altogether() {
    let wide = Unforced::<Wide> {
        value: u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
        tag: 9,
    };
    let bytes: Unforced<Bytes> = wide.morph_with(u64::to_ne_bytes);
    assert_eq!(bytes.value, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(bytes.tag, 9);
}

// --- without `unsafe_transmute`, `repr` is left as written ---

#[typestate(state = S)]
#[repr(Rust)]
struct Unpinned<S: Meta> {
    value: S::Value,
    tag: u8,
}

#[test]
fn morph_only_structs_keep_their_own_repr() {
    let big: Unpinned<Big> =
        Unpinned::<Small> { value: 7, tag: 9 }.morph_with(|v| v as i32);
    assert_eq!((big.value, big.tag), (7, 9));
}
