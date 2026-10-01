//! `#[state]` on generic structs, and `#[typestate]` finding the state
//! among other generic parameters.

use core::{fmt::Debug, marker::PhantomData};
use typestate_groups::{State, WithState};
use typestate_groups_macros::{state, typestate};

#[state]
struct Generic<T> {
    _data: T,
}

#[state]
struct Bounded<T, U>
where
    T: Clone,
{
    _a: T,
    _b: U,
}

#[state]
#[derive(Debug)]
struct StateA;

#[typestate(state = S)]
struct Middle<A, S, B> {
    _a: A,
    _s: PhantomData<S>,
    _b: B,
}

#[typestate(state = S)]
struct Buffered<S, const N: usize> {
    _s: PhantomData<S>,
    _buf: [u8; N],
}

#[typestate(state = S)]
struct WithBound<S: Debug> {
    _s: S,
}

#[typestate(state = S)]
struct WithWhere<S>
where
    S: Debug,
    Option<S>: Debug,
{
    _s: S,
}

#[typestate]
struct Inferred<S> {
    _s: PhantomData<S>,
}

fn is_state<S: State>() {}

fn has_state_a<T: WithState<State = StateA>>() {}

#[test]
fn state_and_typestate_accept_other_generics() {
    is_state::<Generic<&str>>();
    is_state::<Bounded<u8, String>>();

    has_state_a::<Middle<u8, StateA, String>>();
    has_state_a::<Buffered<StateA, 4>>();
    has_state_a::<WithBound<StateA>>();
    has_state_a::<WithWhere<StateA>>();
    has_state_a::<Inferred<StateA>>();
}
