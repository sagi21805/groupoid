//! Safe bit-reinterpreting transitions, checked field by field.

use crate::{State, TransmutableState};

/// Every valid `Src` is a valid `Self` of the same size.
///
/// Implemented for every `Src: IntoBytes` and `Self: FromBytes`, from
/// [`zerocopy`]. `#[typestate]` requires it of every field that projects
/// through the state.
///
/// # Safety
///
/// The bytes of every valid `Src` must form a valid `Self` whenever both
/// have the same size.
#[diagnostic::on_unimplemented(
    message = "convert with `morph`: `{Src}` may hold bits that are not \
               a valid `{Self}`",
    note = "a cast needs `{Src}: zerocopy::IntoBytes` (no padding) and \
            `{Self}: zerocopy::FromBytes` (every bit pattern valid)"
)]
pub unsafe trait CastFrom<Src> {}

// SAFETY: `IntoBytes` gives `S` no uninitialized bytes, and `FromBytes`
// accepts every initialized bit pattern as a `D`.
#[diagnostic::do_not_recommend]
unsafe impl<S: zerocopy::IntoBytes, D: zerocopy::FromBytes> CastFrom<S>
    for D
{
}

/// Every valid `Src` is a valid `Self` of the same size, and neither
/// holds an `UnsafeCell`.
///
/// Implemented for every `Src: IntoBytes + Immutable` and
/// `Self: FromBytes + Immutable`, from [`zerocopy`]. `#[typestate]`
/// requires it of every field that projects through the state for
/// [`cast_state_ref`](crate::Isomorphic::cast_state_ref).
///
/// # Safety
///
/// `Self` must meet [`CastFrom<Src>`], and neither `Self` nor `Src` may
/// hold an `UnsafeCell`.
#[diagnostic::on_unimplemented(
    message = "cast by value with `cast_state` or `cast_state_mut`: \
               `{Src}` or `{Self}` may hold an `UnsafeCell` that `&` \
               could write through",
    note = "a cast through `&` also needs `{Src}` and `{Self}` to be \
            `zerocopy::Immutable`"
)]
pub unsafe trait CastRefFrom<Src>: CastFrom<Src> {}

// SAFETY: `CastFrom` proves the bits valid, and `Immutable` rules out an
// `UnsafeCell` in either type.
#[diagnostic::do_not_recommend]
unsafe impl<S, D> CastRefFrom<S> for D
where
    S: zerocopy::IntoBytes + zerocopy::Immutable,
    D: zerocopy::FromBytes + zerocopy::Immutable,
{
}

/// How a cast holds the container: [`Owned`], [`Shared`] or
/// [`Exclusive`].
pub trait Access {}

/// A cast by value, with [`cast_state`](crate::Isomorphic::cast_state).
pub enum Owned {}

/// A cast through `&`, with
/// [`cast_state_ref`](crate::Isomorphic::cast_state_ref).
pub enum Shared {}

/// A cast through `&mut`, with
/// [`cast_state_mut`](crate::Isomorphic::cast_state_mut).
pub enum Exclusive {}

impl Access for Owned {}
impl Access for Shared {}
impl Access for Exclusive {}

/// `Self` can be reinterpreted in state `To` through access `A` without
/// `unsafe`.
///
/// `#[typestate(unsafe_transmute = true)]` implements it for each access,
/// requiring of every field type `S::P` that projects through the state:
///
/// | `A` | Bounds |
/// |---|---|
/// | [`Owned`] | `To::P: CastFrom<S::P>` |
/// | [`Exclusive`] | `To::P: CastFrom<S::P>`, `S::P: CastFrom<To::P>` |
/// | [`Shared`] | `To::P: CastRefFrom<S::P>` |
///
/// `&mut` needs both directions because the source sees what the target
/// wrote once the borrow ends. `&` rules out `UnsafeCell` because a shared
/// `Cell` viewed as another type could be written through the alias.
///
/// # Safety
///
/// Every valid `Self` must be a valid `Target`, and for [`Exclusive`]
/// every valid `Target` a valid `Self`. For [`Shared`], no field whose
/// type changes may hold an `UnsafeCell` in either state.
#[diagnostic::on_unimplemented(
    message = "add `unsafe_transmute = true` to the `#[typestate]` of \
               `{Self}` to cast it into state `{To}`",
    note = "or convert by value with `morph`"
)]
pub unsafe trait CastableState<To: State, A: Access>:
    TransmutableState<To>
{
}
