// `transmute_state` and its ref/mut forms between same-sized states of
// one struct.
use groupoid::Isomorphic;
use groupoid_macros::{group, state, template, typestate};

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

#[typestate(state = S, unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Value,
}

#[test]
fn transmute_state_between_same_sized_states_of_the_same_struct() {
    let small = Wrap::<Small> {
        value: 0xdead_beefu32,
    };
    let big = unsafe { small.transmute_state::<Big>() };
    let big: Wrap<Big> = big;
    assert_eq!(big.value, 0xdead_beefu32 as i32);
}

#[test]
fn transmute_state_ref_and_mut_round_trip() {
    let mut small = Wrap::<Small> { value: 7 };
    {
        let big_ref: &Wrap<Big> =
            unsafe { small.transmute_state_ref::<Big>() };
        assert_eq!(big_ref.value, 7);
    }
    {
        let big_mut: &mut Wrap<Big> =
            unsafe { small.transmute_state_mut::<Big>() };
        big_mut.value = 9;
    }
    assert_eq!(small.value, 9);
}

// The target state can also be left to inference from the expected type:
// `Target` is a projection through the single `TransmutableState` impl
// `#[typestate]` generates, so unifying it with `Wrap<Big>` settles `To`.
#[test]
fn target_state_inferred_from_expected_type() {
    let small = Wrap::<Small> { value: 3 };
    let big: Wrap<Big> = unsafe { small.transmute_state() };
    assert_eq!(big.value, 3);
}

// A parenthesised projection, or one substituted through a `macro_rules!`
// `$ty` (which reaches the attribute wrapped in an invisible group), is
// still a bare projection.
#[typestate(state = S, unsafe_transmute = true)]
#[allow(unused_parens)]
struct Paren<S: Meta> {
    value: (S::Value),
}

macro_rules! wrap_via_macro {
    ($ty:ty) => {
        #[typestate(state = S, unsafe_transmute = true)]
        struct ViaMacro<S: Meta> {
            value: $ty,
        }
    };
}
wrap_via_macro!(S::Value);

#[test]
fn transmute_state_sees_through_parens_and_macro_groups() {
    let big: Paren<Big> =
        unsafe { Paren::<Small> { value: 3 }.transmute_state::<Big>() };
    assert_eq!(big.value, 3);

    let big: ViaMacro<Big> =
        unsafe { ViaMacro::<Small> { value: 5 }.transmute_state::<Big>() };
    assert_eq!(big.value, 5);
}
