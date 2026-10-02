//! Safe bit-reinterpreting transitions, checked field by field.

use crate::{
    Indirect, LentPointee, Repointed, SharedPointee, State,
    TransmutableState, UniquePointee, UnknownPointee,
};

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

/// Neither `Src` nor `Self` holds an `UnsafeCell`, so a `&Src` can be
/// read as a `&Self`.
///
/// Implemented for every `Src: Immutable` and `Self: Immutable`, from
/// [`zerocopy`]. `#[typestate]` requires it next to [`CastFrom`] of every
/// field that projects through the state for
/// [`cast_state_ref`](crate::Isomorphic::cast_state_ref).
///
/// # Safety
///
/// Neither `Self` nor `Src` may hold an `UnsafeCell`.
#[diagnostic::on_unimplemented(
    message = "convert with `morph`, or cast a field held by value with \
               `cast_state` or `cast_state_mut`: `{Src}` or `{Self}` may \
               hold an `UnsafeCell` that a shared alias could write \
               through",
    note = "a cast through `&`, or of a `&` or raw pointer's pointee, \
            also needs `{Src}` and `{Self}` to be `zerocopy::Immutable`"
)]
pub unsafe trait CastRefFrom<Src> {}

// SAFETY: `Immutable` rules out an `UnsafeCell` in either type.
#[diagnostic::do_not_recommend]
unsafe impl<S: zerocopy::Immutable, D: zerocopy::Immutable> CastRefFrom<S>
    for D
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

/// Every valid `Src` is a valid `Self` when a container holding it is
/// cast through `A`.
///
/// | `A` | Needs |
/// |---|---|
/// | [`Owned`] | `Self: CastFrom<Src>` |
/// | [`Shared`] | also `Self: CastRefFrom<Src>` |
/// | [`Exclusive`] | also `Src: CastFrom<Self>` |
///
/// `&mut` needs both directions because the source sees what the target
/// wrote once the borrow ends. `&` rules out `UnsafeCell` because a shared
/// `Cell` viewed as another type could be written through the alias.
///
/// # Safety
///
/// Every valid `Src` must be a valid `Self`. For [`Shared`], neither may
/// hold an `UnsafeCell`, and for [`Exclusive`] every valid `Self` must be
/// a valid `Src`.
#[doc(hidden)]
pub unsafe trait CastValid<Src, A: Access> {}

// SAFETY: `CastFrom` proves every valid `S` a valid `D`.
unsafe impl<S, D: CastFrom<S>> CastValid<S, Owned> for D {}

// SAFETY: as for `Owned`, and `CastRefFrom` rules out an `UnsafeCell`.
unsafe impl<S, D: CastFrom<S> + CastRefFrom<S>> CastValid<S, Shared>
    for D
{
}

// SAFETY: `CastFrom` proves it both ways.
unsafe impl<S: CastFrom<D>, D: CastFrom<S>> CastValid<S, Exclusive> for D {}

/// The pointee `Src` stays valid as `Dst` when a container holding a
/// pointer with this [`Aliasing`](crate::Aliasing) is cast through `A`.
///
/// | Aliasing | Checks the pointee as |
/// |---|---|
/// | [`UniquePointee`] | `A` |
/// | [`SharedPointee`] | [`Shared`] |
/// | [`LentPointee`] | [`Shared`] under `Shared`, else [`Exclusive`] |
/// | [`UnknownPointee`] | both [`Shared`] and [`Exclusive`] |
///
/// # Safety
///
/// Every valid `Src` must be a valid `Dst` for everyone the aliasing
/// lets see the pointee.
#[doc(hidden)]
pub unsafe trait CastPointee<Src, Dst, A: Access> {}

// SAFETY: only the container reaches the pointee.
unsafe impl<S, D: CastValid<S, A>, A: Access> CastPointee<S, D, A>
    for UniquePointee
{
}

// SAFETY: others may read the pointee through shared aliases.
unsafe impl<S, D: CastValid<S, Shared>, A: Access> CastPointee<S, D, A>
    for SharedPointee
{
}

// SAFETY: the lender sees what the target writes once the borrow ends.
unsafe impl<S, D: CastValid<S, Exclusive>> CastPointee<S, D, Owned>
    for LentPointee
{
}

// SAFETY: through `&`, neither side writes the pointee.
unsafe impl<S, D: CastValid<S, Shared>> CastPointee<S, D, Shared>
    for LentPointee
{
}

// SAFETY: the lender sees what the target writes once the borrow ends.
unsafe impl<S, D: CastValid<S, Exclusive>> CastPointee<S, D, Exclusive>
    for LentPointee
{
}

// SAFETY: anyone may read or write the pointee.
unsafe impl<
    S,
    D: CastValid<S, Shared> + CastValid<S, Exclusive>,
    A: Access,
> CastPointee<S, D, A> for UnknownPointee
{
}

/// `Src`'s pointee stays valid as `Self`'s when a container holding it
/// is cast through `A`, as `Src`'s [`Aliasing`](crate::Aliasing) decides
/// through [`CastPointee`].
///
/// # Safety
///
/// `Self` must be `Src` pointing at another type, whose pointee every
/// valid pointee of `Src` is, for everyone who may see it.
/// [`SAME_POINTEE_LAYOUT`](CastIndirect::SAME_POINTEE_LAYOUT) must be
/// `true` only when both pointees share a size and alignment.
#[doc(hidden)]
pub unsafe trait CastIndirect<Src: Indirect, A: Access>:
    Repointed<Src>
{
    /// Whether both pointees share a size and alignment, which
    /// `CastableState::POINTEE_CHECK` asserts.
    const SAME_POINTEE_LAYOUT: bool;
}

// SAFETY: `CastPointee` proves the pointee valid under `S`'s aliasing,
// and the const compares the pointees' layouts.
unsafe impl<S: Indirect, D: Repointed<S>, A: Access> CastIndirect<S, A>
    for D
where
    S::Aliasing: CastPointee<S::Pointee, D::Pointee, A>,
{
    const SAME_POINTEE_LAYOUT: bool = size_of::<S::Pointee>()
        == size_of::<D::Pointee>()
        && align_of::<S::Pointee>() == align_of::<D::Pointee>();
}

/// `Self` can be reinterpreted in state `To` through access `A` without
/// `unsafe`.
///
/// `#[typestate(unsafe_transmute = true)]` implements it for each access.
/// It requires [`CastValid<S::P, A>`](CastValid) of each field `S::P`,
/// and [`CastIndirect<F, A>`](CastIndirect) of each [`Indirect`] field
/// `F`, whose pointee gets the checks its
/// [`Aliasing`](Indirect::Aliasing) asks for. Every pointee must also
/// keep its size and alignment, which
/// [`POINTEE_CHECK`](CastableState::POINTEE_CHECK) asserts.
///
/// # Safety
///
/// Every valid `Self` must be a valid `Target`, and for [`Exclusive`]
/// every valid `Target` a valid `Self`. For [`Shared`], no field whose
/// type changes may hold an `UnsafeCell` in either state. Every pointee
/// must stay valid in the same way for every alias that may see it.
#[diagnostic::on_unimplemented(
    message = "add `unsafe_transmute = true` to the `#[typestate]` of \
               `{Self}` to cast it into state `{To}`",
    note = "or convert by value with `morph`"
)]
pub unsafe trait CastableState<To: State, A: Access>:
    TransmutableState<To>
{
    /// Compile-time check that every pointee keeps its size and
    /// alignment in state `To`. `#[typestate]` overrides it with one
    /// assertion per pointer field.
    const POINTEE_CHECK: () = ();
}
