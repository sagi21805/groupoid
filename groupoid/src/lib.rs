#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[macro_use]
mod macros;

mod error_helpers;
mod layout;
mod morph;
mod transmute;

pub use error_helpers::PinnedLayout;
pub use groupoid_macros::*;
pub use layout::{
    PinnedTypeAlignment, PinnedTypeLayout, PinnedTypeSize, SameAlignment,
    SameLayout, SameSize, TypeAlignment, TypeLayout, TypeSize,
    UnpinnedTypeAlignment, UnpinnedTypeLayout, UnpinnedTypeSize,
};
pub use morph::Morph;
pub use transmute::{Isomorphic, TransmutableState};

/// A type that represents a state of an object.
pub trait State {}

/// A type that has a state.
pub trait WithState {
    type State: State;
}

/// A type that captures a specific implementation of a [`group_trait`].
pub trait Group {}
