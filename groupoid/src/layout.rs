//! Layout markers `#[group]` records and the traits that compare them.

use core::marker::PhantomData;

/// What `#[group]` knows of an associated type's layout.
#[doc(hidden)]
pub trait TypeLayout {
    type Size: TypeSize;
    type Alignment: TypeAlignment;
}

/// Layout of type `T` in group `G`, with `#[size(N)]`. `G` and `T` only
/// name the type in errors.
#[doc(hidden)]
pub struct PinnedTypeLayout<G, T, const SIZE: usize, const ALIGN: usize>(
    PhantomData<(G, T)>,
);

impl<G, T, const SIZE: usize, const ALIGN: usize> TypeLayout
    for PinnedTypeLayout<G, T, SIZE, ALIGN>
{
    type Size = PinnedTypeSize<G, T, SIZE>;
    type Alignment = PinnedTypeAlignment<G, T, ALIGN>;
}

/// Layout of type `T` in group `G`, without `#[size(N)]`.
#[doc(hidden)]
pub struct UnpinnedTypeLayout<G, T>(PhantomData<(G, T)>);

impl<G, T> TypeLayout for UnpinnedTypeLayout<G, T> {
    type Size = UnpinnedTypeSize<G, T>;
    type Alignment = UnpinnedTypeAlignment<G, T>;
}

layout_property! {
    "size": TypeSize, PinnedTypeSize<SIZE>, UnpinnedTypeSize,
    #[diagnostic::on_unimplemented(message = "convert with `morph_with` \
                                              instead of transmuting: \
                                              `{Other}` and `{Self}` \
                                              differ in size")]
    SameSize
}

layout_property! {
    "alignment": TypeAlignment, PinnedTypeAlignment<ALIGN>,
    UnpinnedTypeAlignment,
    #[diagnostic::on_unimplemented(message = "add `align = N` to \
                                              `#[typestate(..)]`, with \
                                              `N` at least the larger \
                                              alignment of `{Other}` and \
                                              `{Self}`")]
    SameAlignment
}

/// `Self` and `Other` describe types of the same size and alignment.
///
/// # Safety
///
/// The described types must have the same size and alignment.
#[doc(hidden)]
pub unsafe trait SameLayout<Other: TypeLayout>: TypeLayout {}

// SAFETY: `SameSize` and `SameAlignment` match the sizes and alignments.
unsafe impl<L1: TypeLayout, L2: TypeLayout> SameLayout<L2> for L1
where
    L1::Size: SameSize<L2::Size>,
    L1::Alignment: SameAlignment<L2::Alignment>,
{
}
