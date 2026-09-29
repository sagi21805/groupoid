//! Declarative macros shared by the crate's modules.

/// Declares one property of a layout that `#[typestate]` compares, such
/// as its size: the marker trait, the pinned and unpinned marker types,
/// and the unsafe `Same*` trait with its impl between equal pinned
/// markers.
///
/// `size: TypeSize, PinnedTypeSize<SIZE>, UnpinnedTypeSize, SameSize` ->
/// `trait TypeSize`, `struct PinnedTypeSize<G, T, const SIZE: usize>`,
/// `struct UnpinnedTypeSize<G, T>`, `unsafe trait SameSize<Other:
/// TypeSize>` and `impl SameSize<PinnedTypeSize<G2, T2, SIZE>> for
/// PinnedTypeSize<G1, T1, SIZE>`
macro_rules! layout_property {
    (
        $property:literal: $marker:ident, $pinned:ident<$value:ident>, $unpinned:ident,
        $(#[$same_attr:meta])*
        $same:ident
    ) => {
        #[doc = concat!("The ", $property, " of a type, as [`", stringify!($same), "`] compares it.")]
        #[doc(hidden)]
        pub trait $marker {}

        #[doc = concat!("The ", $property, " of type `T` of group `G`, as [`", stringify!($same), "`] compares it.")]
        #[doc(hidden)]
        pub struct $pinned<G, T, const $value: usize>(PhantomData<(G, T)>);

        impl<G, T, const $value: usize> $marker for $pinned<G, T, $value> {}

        #[doc = concat!("The ", $property, " of type `T` of group `G`, which has no `#[size(N)]`.")]
        #[doc(hidden)]
        pub struct $unpinned<G, T>(PhantomData<(G, T)>);

        impl<G, T> $marker for $unpinned<G, T> {}

        #[doc = concat!("The type `Self` describes has the same ", $property, " as the type `Other` describes.")]
        $(#[$same_attr])*
        ///
        /// # Safety
        ///
        #[doc = concat!("The types `Self` and `Other` describe must have the same ", $property, ".")]
        /// `#[typestate]` transmutes between them on that promise.
        #[doc(hidden)]
        pub unsafe trait $same<Other: $marker>: $marker {}

        // SAFETY: both carry the same value, which `#[group]` records
        // from their type.
        #[diagnostic::do_not_recommend]
        unsafe impl<G1, T1, G2, T2, const $value: usize>
            $same<$pinned<G2, T2, $value>> for $pinned<G1, T1, $value>
        {
        }
    };
}
