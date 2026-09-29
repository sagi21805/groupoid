//! The layout markers `#[group]` records for associated types, and the
//! traits that compare them before a transmute.

use core::marker::PhantomData;

/// What `#[group]` knows of an associated type's layout: a
/// [`PinnedTypeLayout`] or an [`UnpinnedTypeLayout`], as [`SameLayout`]
/// compares it.
#[doc(hidden)]
pub trait TypeLayout {
    type Size: TypeSize;
    type Alignment: TypeAlignment;
}

/// The layout `#[group]` records for an associated type with
/// `#[size(N)]`: type `T` of group `G`, `SIZE` bytes, `ALIGN`-aligned.
///
/// `G` and `T` appear only so errors can name them.
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

/// Records that an associated type has no `#[size(N)]`, so `#[group]`
/// knows nothing of its layout.
#[doc(hidden)]
pub struct UnpinnedTypeLayout<G, T>(PhantomData<(G, T)>);

impl<G, T> TypeLayout for UnpinnedTypeLayout<G, T> {
    type Size = UnpinnedTypeSize<G, T>;
    type Alignment = UnpinnedTypeAlignment<G, T>;
}

layout_property! {
    "size": TypeSize, PinnedTypeSize<SIZE>, UnpinnedTypeSize,
    /// [`SameLayout`] requires it, and `#[typestate(align = N)]` requires
    /// it alone.
    #[diagnostic::on_unimplemented(message = "convert with `morph_with` \
                                              instead of transmuting: \
                                              `{Other}` and `{Self}` \
                                              differ in size")]
    SameSize
}

layout_property! {
    "alignment": TypeAlignment, PinnedTypeAlignment<ALIGN>,
    UnpinnedTypeAlignment,
    /// [`SameLayout`] requires it once the sizes match.
    #[diagnostic::on_unimplemented(message = "add `align = N` to \
                                              `#[typestate(..)]`, with \
                                              `N` at least the larger \
                                              alignment of `{Other}` and \
                                              `{Self}`")]
    SameAlignment
}

/// The type `Self` describes has the same size and alignment as the type
/// `Other` describes. `#[typestate]` requires it between two states'
/// layouts to transmute, unless `align = N` forces the alignment.
///
/// # Safety
///
/// The types `Self` and `Other` describe must have the same size and
/// alignment. `#[typestate]` transmutes between them on that promise.
#[doc(hidden)]
pub unsafe trait SameLayout<Other: TypeLayout>: TypeLayout {}

// SAFETY: `SameSize` and `SameAlignment` match the sizes and alignments.
unsafe impl<L1: TypeLayout, L2: TypeLayout> SameLayout<L2> for L1
where
    L1::Size: SameSize<L2::Size>,
    L1::Alignment: SameAlignment<L2::Alignment>,
{
}
