use std::marker::PhantomData;

use groupoid_macros::{blueprint, group, state, typestate};

// --- #[state] on generic structs
// -------------------------------------------------

#[state]
struct GenericState<T> {
    #[allow(dead_code)]
    data: T,
}

#[state]
struct MultiGenericState<T, U>
where
    T: Clone,
{
    #[allow(dead_code)]
    a: T,
    #[allow(dead_code)]
    b: U,
}

fn accepts_state<S: groupoid::State>(_: &S) {}

#[test]
fn generic_state_satisfies_state_for_any_t() {
    accepts_state(&GenericState { data: 5u32 });
    accepts_state(&GenericState { data: "hi" });
}

#[test]
fn state_impl_respects_where_clause() {
    accepts_state(&MultiGenericState { a: 1u8, b: 2u8 });
}

// --- multiple #[state] structs carrying independent behavior
// ---------------------

#[state]
struct StateA;
#[state]
struct StateB;

trait Describe {
    fn name() -> &'static str;
}

impl Describe for StateA {
    fn name() -> &'static str {
        "StateA"
    }
}

impl Describe for StateB {
    fn name() -> &'static str {
        "StateB"
    }
}

#[test]
fn multiple_states_carry_independent_behavior() {
    assert_eq!(StateA::name(), "StateA");
    assert_eq!(StateB::name(), "StateB");
}

// --- #[typestate] with the state param in the middle of the generic list
// ---------

#[typestate(state = S)]
struct Multi<A, S, B> {
    #[allow(dead_code)]
    a: A,
    #[allow(dead_code)]
    s: PhantomData<S>,
    #[allow(dead_code)]
    b: B,
}

fn same_state<T: groupoid::WithState<State = StateA>>() {}

#[test]
fn typestate_matches_non_first_generic_param() {
    same_state::<Multi<u8, StateA, String>>();
}

// --- #[typestate] preserving an existing where-clause on the state param
// ---------

#[typestate(state = S)]
struct WithWhere<S: std::fmt::Debug> {
    #[allow(dead_code)]
    s: S,
}

fn assert_with_state<T: groupoid::WithState>() {}

#[state]
struct DebugState;

impl std::fmt::Debug for DebugState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DebugState")
    }
}

#[test]
fn typestate_preserves_where_clause() {
    // Compiles only if `impl WithState` keeps the `S: Debug` bound.
    assert_with_state::<WithWhere<DebugState>>();
}

// --- #[typestate] alongside a const generic
// --------------------------------------

#[typestate(state = S)]
struct Buffered<S, const N: usize> {
    #[allow(dead_code)]
    s: PhantomData<S>,
    #[allow(dead_code)]
    buf: [u8; N],
}

#[test]
fn typestate_ignores_const_generics() {
    same_state::<Buffered<StateA, 4>>();
}

// --- #[typestate] with no `state = ..` infers the sole generic type param
// --------

#[typestate]
struct Inferred<S> {
    #[allow(dead_code)]
    s: PhantomData<S>,
}

#[test]
fn typestate_infers_sole_generic_as_state() {
    same_state::<Inferred<StateA>>();
}

// --- SizedWithState derives despite an extra, non-blueprint bound
// ----------------

#[blueprint]
trait Sized4 {
    type Value;
}

#[state]
struct Sized4State;

impl std::fmt::Debug for Sized4State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sized4State")
    }
}

#[group(Sized4Group)]
impl Sized4 for (Sized4State,) {
    #[size(4)]
    type Value = u32;
}

// Extra bounds next to the blueprint trait still derive `SizedWithState`.
#[typestate(state = S, unsafe_transmute = true)]
struct SizedWrap<S: Sized4 + std::fmt::Debug> {
    #[allow(dead_code)]
    value: S::Value,
}

fn assert_sized_with_state<
    T: groupoid::SizedWithState<N, A>,
    const N: usize,
    const A: usize,
>() {
}

#[test]
fn sized_with_state_derives_despite_extra_non_blueprint_bound() {
    // `u32` is 4 bytes and 4-aligned, so both halves of the layout key are
    // 4.
    assert_sized_with_state::<SizedWrap<Sized4State>, 4, 4>();
}
