//! The by-value transition `#[typestate]` generates next to the in-place
//! transmute: `restate_with` (safe, the caller converts each projection)
//! and `unsafe fn restate` (each projection is bit-reinterpreted under the
//! shared size pin). Both rebuild the struct field by field, so they exist
//! for every struct that projects through its state, including the shapes
//! that disqualify the in-place path (`Option<S::Value>` and friends).
//!
//! The macro sees through `Option`, arrays, `Box` and tuples by itself; any
//! other wrapper goes through a user `Restate` impl.

use core::marker::PhantomData;
use groupoid::Restate;
use groupoid_macros::{blueprint, group, state, typestate};

#[blueprint]
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

// --- a wrapper the macro cannot see through, restated by hand -----------

#[derive(Debug, PartialEq)]
struct Pair<T>(T, T);

impl<A, B> Restate<A, B> for Pair<A> {
    type Output = Pair<B>;

    fn restate(self, leaf: &mut impl FnMut(A) -> B) -> Pair<B> {
        Pair(leaf(self.0), leaf(self.1))
    }
}

// --- every shape at once --------------------------------------------------

#[typestate(state = S)]
struct Wrap<S: Meta> {
    value: S::Value,
    maybe: Option<S::Value>,
    many: [S::Value; 2],
    boxed: Box<S::Value>,
    pair: (S::Value, u8),
    nested: Option<[S::Value; 2]>,
    user: Pair<S::Value>,
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
    assert_eq!(big.tag, 12, "the state-independent field is moved as-is");
}

#[test]
fn restate_reinterprets_every_projection_under_the_size_pin() {
    let big: Wrap<Big> = unsafe { sample().restate::<Big, _>() };
    assert_round_trip(big);
}

#[test]
fn restate_with_converts_every_projection_through_the_leaf() {
    let mut calls = 0;
    let big: Wrap<Big> = sample().restate_with(|v| {
        calls += 1;
        v as i32
    });
    assert_round_trip(big);
    assert_eq!(
        calls, 10,
        "one call per projection: value, maybe, many x2, boxed, pair.0, nested x2, user x2"
    );
}

#[test]
fn restate_with_sees_none_without_calling_the_leaf() {
    let small = Wrap::<Small> {
        maybe: None,
        nested: None,
        ..sample()
    };
    let big: Wrap<Big> = small.restate_with(|v| v as i32);
    assert_eq!(big.maybe, None);
    assert_eq!(big.nested, None);
}

// --- the `Option` field would be rejected by `unsafe_transmute = true`,
// yet `restate` exists ---

#[typestate(state = S)]
struct NotTransmutable<S: Meta> {
    value: Option<S::Value>,
}

#[test]
fn a_struct_without_the_in_place_path_still_restates_by_value() {
    let big: NotTransmutable<Big> = unsafe {
        NotTransmutable::<Small> {
            value: Some(0xdead_beef),
        }
        .restate::<Big, 4>()
    };
    assert_eq!(big.value, Some(0xdead_beefu32 as i32));
}

// --- other generics ride along, and tuple structs are positional ---------

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
    let big: Ordered<Big, u64> = small.restate_with(|v| v as i32);
    assert_eq!((big.head, big.value, big.tail), (1, 2, 3));
}

#[typestate(state = S)]
struct Positional<S: Meta>(u8, S::Value, Option<S::Value>);

#[test]
fn tuple_structs_are_rebuilt_positionally() {
    let big: Positional<Big> = unsafe { Positional::<Small>(1, 2, Some(3)).restate::<Big, 4>() };
    assert_eq!((big.0, big.1, big.2), (1, 2, Some(3)));
}

// --- forced alignment: `restate` is bounded on size alone ----------------

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
fn forced_alignment_mode_still_generates_restate() {
    let wide = Forced::<Wide> {
        value: u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
        tag: 9,
    };
    let bytes: Forced<Bytes> = unsafe { wide.restate::<Bytes, _>() };
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
fn restate_ignores_alignment_altogether() {
    let wide = Unforced::<Wide> {
        value: u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
        tag: 9,
    };
    let bytes: Unforced<Bytes> = unsafe { wide.restate::<Bytes, 8>() };
    assert_eq!(bytes.value, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(bytes.tag, 9);
}

// --- without `unsafe_transmute = true` the struct's attributes are left
// alone: no `repr(C)` is injected, and an explicit `repr(Rust)` - which
// the in-place path would reject - is fine, since nothing reasons about
// the layout ---

#[typestate(state = S)]
#[repr(Rust)]
struct Unpinned<S: Meta> {
    value: S::Value,
    tag: u8,
}

#[test]
fn restate_only_structs_keep_their_own_repr() {
    let big: Unpinned<Big> = Unpinned::<Small> { value: 7, tag: 9 }.restate_with(|v| v as i32);
    assert_eq!((big.value, big.tag), (7, 9));
}
