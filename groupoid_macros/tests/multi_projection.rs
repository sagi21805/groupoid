// A template with several associated types: three pinned, one not.

use groupoid::{Isomorphic, State};
use groupoid_macros::{
    group, group_impl, group_trait, state, template, typestate,
};

#[template]
trait Meta {
    type Type1;
    type Type2;
    type Type3;
    type Loose;
}

#[state]
#[derive(Debug, PartialEq)]
struct StateA;
#[state]
#[derive(Debug, PartialEq)]
struct StateB;

#[group(AGroup)]
impl Meta for (StateA,) {
    #[size(4)]
    type Type1 = u32;
    #[size(8)]
    type Type2 = u64;
    #[size(2)]
    type Type3 = u16;
    type Loose = String;
}

#[group(BGroup)]
impl Meta for (StateB,) {
    #[size(4)]
    type Type1 = f32;
    #[size(8)]
    type Type2 = i64;
    #[size(2)]
    type Type3 = i16;
    type Loose = Vec<u8>;
}

#[typestate(state = S, unsafe_transmute = true)]
#[derive(Debug, PartialEq)]
struct Triple<S: Meta> {
    a: S::Type1,
    b: S::Type2,
    c: S::Type3,
}

#[typestate(state = S)]
struct Mixed<S: Meta> {
    a: S::Type1,
    loose: S::Loose,
}

#[typestate(state = S)]
struct Pairs<S: Meta> {
    pairs: Vec<(S::Type1, S::Type2)>,
}

#[test]
fn three_projections_transmute_round_trip() {
    let a = Triple::<StateA> { a: 1, b: 2, c: 3 };

    // SAFETY: every bit pattern is a valid `f32`, `i64` and `i16`.
    let b = unsafe { a.transmute_state::<StateB>() };
    assert_eq!(b.a.to_bits(), 1);
    assert_eq!((b.b, b.c), (2, 3));

    // SAFETY: every bit pattern is a valid `u32`, `u64` and `u16`.
    let back = unsafe { b.transmute_state::<StateA>() };
    assert_eq!(back, Triple { a: 1, b: 2, c: 3 });
}

#[test]
fn three_projections_morph_with_one_closure_each() {
    let a = Triple::<StateA> { a: 7, b: 9, c: 11 };

    let b: Triple<StateB> = a.morph_with(TripleMorph {
        type1: &mut |x| x.min(4) as f32,
        type2: &mut |x| -(x as i64),
        type3: &mut |x| x as i16,
    });

    assert_eq!((b.a, b.b, b.c), (4.0, -9, 11));
}

#[test]
fn unpinned_projection_morphs() {
    let a = Mixed::<StateA> {
        a: 1,
        loose: "hi".to_owned(),
    };

    let b: Mixed<StateB> = a.morph_with(MixedMorph {
        type1: &mut |x| x as f32,
        loose: &mut String::into_bytes,
    });

    assert_eq!((b.a, b.loose), (1.0, b"hi".to_vec()));
}

#[test]
fn tuple_inside_wrapper_morphs_each_element_through_its_projection() {
    let a = Pairs::<StateA> {
        pairs: vec![(1, 2), (3, 4)],
    };
    let mut calls = 0;

    let b: Pairs<StateB> = a.morph_with(PairsMorph {
        type1: &mut |x| {
            calls += 1;
            x as f32
        },
        type2: &mut |x| -(x as i64),
    });

    assert_eq!(b.pairs, vec![(1.0, -2), (3.0, -4)]);
    assert_eq!(calls, 2);
}

#[group_trait(by = Meta)]
trait Describe {
    fn describe(&self) -> String;
}

#[group_impl(AGroup)]
impl<S: Meta + State> Describe for Triple<S> {
    fn describe(&self) -> String {
        let (a, b, c): (u32, u64, u16) = (self.a, self.b, self.c);
        format!("{a} {b} {c}")
    }
}

#[group_impl(BGroup)]
impl<S: Meta + State> Describe for Triple<S> {
    fn describe(&self) -> String {
        let (a, b, c): (f32, i64, i16) = (self.a, self.b, self.c);
        format!("{a:.1} {b} {c}")
    }
}

#[test]
fn group_impl_sees_every_projection_as_its_concrete_type() {
    let a = Triple::<StateA> { a: 1, b: 2, c: 3 };
    let b = Triple::<StateB> {
        a: 1.5,
        b: -2,
        c: -3,
    };

    assert_eq!(
        (a.describe(), b.describe()),
        ("1 2 3".into(), "1.5 -2 -3".into())
    );
}
