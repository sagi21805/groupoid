#![no_std]

#[macro_use]
mod macros;

mod error_helpers;
mod layout;
mod morph;
mod transmute;

pub use error_helpers::PinnedLayout;
pub use layout::{
    PinnedTypeAlignment, PinnedTypeLayout, PinnedTypeSize, SameAlignment,
    SameLayout, SameSize, TypeAlignment, TypeLayout, TypeSize,
    UnpinnedTypeAlignment, UnpinnedTypeLayout, UnpinnedTypeSize,
};
pub use morph::{MorphFrom, Morphic, TryMorphFrom};
pub use transmute::{Isomorphic, TransmutableState};
pub use typestate_groups_macros::*;

/// A type that represents a state of an object.
pub trait State {}

/// A type that has a state.
pub trait WithState {
    type State: State;
}

/// `Self` with its state replaced by `To` and every other generic
/// unchanged.
///
/// `#[typestate]` implements it for every target state.
pub trait Restate<To: State>: WithState {
    /// `Self` in state `To`.
    type Target: WithState<State = To>;
}

/// A type that captures a specific implementation of a [`group_trait`].
pub trait Group {}
