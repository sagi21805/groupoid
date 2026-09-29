//! Helper traits for prettier error messages.
//!
//! Nothing here decides which transmutes compile: the impls in `layout`
//! accept equal sizes and alignments and reject everything else without
//! them.
//! These items only steer which failing bound rustc reports, so the error
//! says what to write instead of a generic "differ in size".

use crate::{
    Group, PinnedTypeAlignment, PinnedTypeSize, SameAlignment, SameSize,
    TypeAlignment, TypeSize, UnpinnedTypeAlignment, UnpinnedTypeSize,
};

/// A bound that never holds: nothing implements it. The impls of
/// [`SameSize`] for an [`UnpinnedTypeSize`] and of [`SameAlignment`] for
/// an [`UnpinnedTypeAlignment`] require `G: PinnedLayout<T>`, so they
/// never apply, and the unmet bound's error asks for `#[size(N)]`.
///
/// # Safety
///
/// Never implement it: an impl for group `G` lets
/// `UnpinnedTypeSize<G, T>` pass [`SameSize`] and
/// `UnpinnedTypeAlignment<G, T>` pass [`SameAlignment`] against any
/// size or alignment.
#[doc(hidden)]
#[diagnostic::on_unimplemented(message = "add `#[size(N)]` to the \
                                          associated type set to `{T}` \
                                          in `#[group({Self})]` to \
                                          transmute it")]
pub unsafe trait PinnedLayout<T>: Group {}

// SAFETY: requires `G2: PinnedLayout<T2>`, and no group implements
// `PinnedLayout`.
unsafe impl<G1, T1, G2: PinnedLayout<T2>, T2, const SIZE: usize>
    SameSize<UnpinnedTypeSize<G2, T2>> for PinnedTypeSize<G1, T1, SIZE>
{
}

// SAFETY: requires `G: PinnedLayout<T>`, and no group implements
// `PinnedLayout`.
unsafe impl<G: PinnedLayout<T>, T, Other: TypeSize> SameSize<Other>
    for UnpinnedTypeSize<G, T>
{
}

// SAFETY: requires `G2: PinnedLayout<T2>`, and no group implements
// `PinnedLayout`.
unsafe impl<G1, T1, G2: PinnedLayout<T2>, T2, const ALIGN: usize>
    SameAlignment<UnpinnedTypeAlignment<G2, T2>>
    for PinnedTypeAlignment<G1, T1, ALIGN>
{
}

// SAFETY: requires `G: PinnedLayout<T>`, and no group implements
// `PinnedLayout`.
unsafe impl<G: PinnedLayout<T>, T, Other: TypeAlignment>
    SameAlignment<Other> for UnpinnedTypeAlignment<G, T>
{
}
