//! `morph` and `try_morph` rebuild a struct for another state through the
//! user's `MorphFrom` and `TryMorphFrom` impls.

use core::marker::PhantomData;
use groupoid::{MorphFrom, Morphic, TryMorphFrom};
use groupoid_macros::{group, state, template, typestate};

#[template]
trait Meta {
    type Value;
}

#[state]
#[derive(Debug, PartialEq)]
struct Small;
#[state]
#[derive(Debug, PartialEq)]
struct Big;

#[group(SmallGroup)]
impl Meta for (Small,) {
    type Value = u8;
}

#[group(BigGroup)]
impl Meta for (Big,) {
    type Value = u64;
}

// --- one generic impl covers every state pair ---

#[typestate]
#[derive(Debug, PartialEq)]
struct Inner<S: Meta> {
    value: S::Value,
}

impl<S: Meta, S2: Meta> MorphFrom<Inner<S>> for Inner<S2>
where
    S2::Value: From<S::Value>,
{
    fn morph_from(src: Inner<S>) -> Self {
        Inner {
            value: src.value.into(),
        }
    }
}

#[test]
fn generic_impl_morphs_between_states() {
    let big = Inner::<Small> { value: 7 }.morph::<Big>();
    assert_eq!(big.value, 7u64);
}

#[test]
fn generic_impl_morphs_to_the_same_state() {
    let small = Inner::<Small> { value: 7 }.morph::<Small>();
    assert_eq!(small.value, 7u8);
}

// --- an outer struct reuses the inner struct's conversion ---

#[typestate]
struct Outer<S: Meta> {
    inner: Inner<S>,
    maybe: Option<Inner<S>>,
    many: Vec<Inner<S>>,
    tag: u8,
}

impl<S: Meta, S2: Meta> MorphFrom<Outer<S>> for Outer<S2>
where
    Inner<S2>: MorphFrom<Inner<S>>,
{
    fn morph_from(src: Outer<S>) -> Self {
        Outer {
            inner: src.inner.morph::<S2>(),
            maybe: src.maybe.map(|inner| inner.morph::<S2>()),
            many: src
                .many
                .into_iter()
                .map(|inner| inner.morph::<S2>())
                .collect(),
            tag: src.tag,
        }
    }
}

#[test]
fn outer_struct_reuses_the_inner_conversion() {
    let small = Outer::<Small> {
        inner: Inner { value: 1 },
        maybe: Some(Inner { value: 2 }),
        many: vec![Inner { value: 3 }, Inner { value: 4 }],
        tag: 5,
    };

    let big = small.morph::<Big>();

    assert_eq!(big.inner, Inner { value: 1 });
    assert_eq!(big.maybe, Some(Inner { value: 2 }));
    assert_eq!(big.many, [Inner { value: 3 }, Inner { value: 4 }]);
    assert_eq!(big.tag, 5);
}

// --- `try_morph` through a fallible impl ---

#[typestate]
#[derive(Debug, PartialEq)]
struct Narrow<S: Meta> {
    value: S::Value,
}

impl<S: Meta, S2: Meta> TryMorphFrom<Narrow<S>> for Narrow<S2>
where
    S2::Value: TryFrom<S::Value>,
{
    type Error = <S2::Value as TryFrom<S::Value>>::Error;

    fn try_morph_from(src: Narrow<S>) -> Result<Self, Self::Error> {
        Ok(Narrow {
            value: src.value.try_into()?,
        })
    }
}

#[test]
fn try_morph_succeeds_when_the_value_fits() {
    let small = Narrow::<Big> { value: 200 }.try_morph::<Small>();
    assert_eq!(small, Ok(Narrow { value: 200u8 }));
}

#[test]
fn try_morph_fails_when_the_value_does_not_fit() {
    let small = Narrow::<Big> { value: 300 }.try_morph::<Small>();
    assert!(small.is_err());
}

// --- a concrete impl for one pair, with other generics carried over ---

#[typestate(state = S)]
struct Ordered<S: Meta, T> {
    head: u16,
    value: S::Value,
    tail: T,
}

impl<T> MorphFrom<Ordered<Small, T>> for Ordered<Big, T> {
    fn morph_from(src: Ordered<Small, T>) -> Self {
        Ordered {
            head: src.head,
            value: u64::from(src.value) * 2,
            tail: src.tail,
        }
    }
}

#[test]
fn concrete_impl_keeps_other_generics() {
    let small = Ordered::<Small, &str> {
        head: 1,
        value: 2,
        tail: "t",
    };
    let big = small.morph::<Big>();
    assert_eq!((big.head, big.value, big.tail), (1, 4, "t"));
}

// --- a struct that only holds the state still morphs ---

#[typestate]
struct Tag<S: Meta>(PhantomData<S>);

impl<S: Meta, S2: Meta> MorphFrom<Tag<S>> for Tag<S2> {
    fn morph_from(_: Tag<S>) -> Self {
        Tag(PhantomData)
    }
}

#[test]
fn tuple_struct_without_projection_morphs() {
    let Tag(PhantomData) = Tag::<Small>(PhantomData).morph::<Big>();
}
