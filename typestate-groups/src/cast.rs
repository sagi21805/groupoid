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

/// `Self` can be reinterpreted in state `To` through access `A` without
/// `unsafe`.
///
/// `#[typestate(unsafe_transmute = true)]` implements it for each access.
/// Each field of type `S::P`, and each pointee `T` of a pointer field,
/// needs the bounds of an access, with `Dst` the type in state `To`:
///
/// | Access | Bounds |
/// |---|---|
/// | [`Owned`] | `Dst: CastFrom<Src>` |
/// | [`Exclusive`] | `Dst: CastFrom<Src>`, `Src: CastFrom<Dst>` |
/// | [`Shared`] | `Dst: CastFrom<Src>`, `Dst: CastRefFrom<Src>` |
///
/// `&mut` needs both directions because the source sees what the target
/// wrote once the borrow ends. `&` rules out `UnsafeCell` because a shared
/// `Cell` viewed as another type could be written through the alias.
///
/// A field `S::P` and a `Box<T>` pointee take the access `A`. Other
/// pointees take the access of whoever else may see them:
///
/// | Field | `Owned` | `Shared` | `Exclusive` |
/// |---|---|---|---|
/// | `S::P`, `Box<T>` | `Owned` | `Shared` | `Exclusive` |
/// | `&T` | `Shared` | `Shared` | `Shared` |
/// | `&mut T` | `Exclusive` | `Shared` | `Exclusive` |
/// | `*const T`, `*mut T`, `NonNull<T>` | both | both | both |
///
/// Every pointee must also keep its size and alignment, which
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
