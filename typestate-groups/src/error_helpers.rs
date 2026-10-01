//! Items that only change which error rustc reports.

use crate::{
    Group, PinnedTypeAlignment, PinnedTypeSize, SameAlignment, SameSize,
    TypeAlignment, TypeSize, UnpinnedTypeAlignment, UnpinnedTypeSize,
};

/// A bound that never holds, so the error on unpinned layouts asks for
/// `#[size(N)]`.
///
/// # Safety
///
/// Never implement it: an impl lets unpinned layouts pass [`SameSize`]
/// and [`SameAlignment`] against any layout.
#[doc(hidden)]
#[diagnostic::on_unimplemented(message = "add `#[size(N)]` to the \
                                          associated type set to `{T}` \
                                          in `#[group({Self})]` to \
                                          transmute it")]
pub unsafe trait PinnedLayout<T>: Group {}

// SAFETY: no group implements `PinnedLayout`.
unsafe impl<G1, T1, G2: PinnedLayout<T2>, T2, const SIZE: usize>
    SameSize<UnpinnedTypeSize<G2, T2>> for PinnedTypeSize<G1, T1, SIZE>
{
}

// SAFETY: no group implements `PinnedLayout`.
unsafe impl<G: PinnedLayout<T>, T, Other: TypeSize> SameSize<Other>
    for UnpinnedTypeSize<G, T>
{
}

// SAFETY: no group implements `PinnedLayout`.
unsafe impl<G1, T1, G2: PinnedLayout<T2>, T2, const ALIGN: usize>
    SameAlignment<UnpinnedTypeAlignment<G2, T2>>
    for PinnedTypeAlignment<G1, T1, ALIGN>
{
}

// SAFETY: no group implements `PinnedLayout`.
unsafe impl<G: PinnedLayout<T>, T, Other: TypeAlignment>
    SameAlignment<Other> for UnpinnedTypeAlignment<G, T>
{
}
