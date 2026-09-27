#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod morph;

use core::marker::PhantomData;

pub use groupoid_macros::*;
pub use morph::Morph;

/// A type that represents a state of an object.
pub trait State {}

/// A type that has a state.
pub trait WithState {
    type State: State;
}

/// A type that captures a specific implementation of a [`group_trait`].
pub trait Group {}

// TODO: Maybe use in the future
// pub trait GroupMarker {
// type Marker: Group;
// }

/// The layout `#[group]` pins an associated type to with `#[size(N)]`:
/// type `T` of group `G`, `SIZE` bytes, `ALIGN`-aligned.
///
/// `G` and `T` appear only so errors can name them.
#[doc(hidden)]
pub struct Layout<G, T, const SIZE: usize, const ALIGN: usize>(
    PhantomData<(G, T)>,
);

/// The layout `#[group]` records for an associated type without
/// `#[size(N)]`. It transmutes into nothing.
#[doc(hidden)]
pub struct Unpinned<G, T>(PhantomData<(G, T)>);

/// The alignment of type `T` of group `G`, as [`SameAlign`] compares it.
#[doc(hidden)]
pub struct Aligned<G, T, const ALIGN: usize>(PhantomData<(G, T)>);

/// Never holds. An [`Unpinned`] layout on either side of [`SameSize`] or
/// [`SameLayout`] requires it of its group, so the error asks for
/// `#[size(N)]`.
#[doc(hidden)]
#[diagnostic::on_unimplemented(message = "add `#[size(N)]` to the \
                                          associated type set to `{T}` \
                                          in `#[group({Self})]` to \
                                          transmute it")]
pub trait Pinned<T> {}

/// Holds when `Self` and `Other` are pinned layouts of the same size.
/// `#[typestate]` requires it under `align = N`.
#[doc(hidden)]
#[diagnostic::on_unimplemented(message = "convert with `morph_with` \
                                          instead of transmuting: \
                                          `{Other}` and `{Self}` differ \
                                          in size")]
pub trait SameSize<Other> {}

impl<G1, T1, G2, T2, const SIZE: usize, const A1: usize, const A2: usize>
    SameSize<Layout<G2, T2, SIZE, A2>> for Layout<G1, T1, SIZE, A1>
{
}

impl<G1, T1, G2: Pinned<T2>, T2, const SIZE: usize, const ALIGN: usize>
    SameSize<Unpinned<G2, T2>> for Layout<G1, T1, SIZE, ALIGN>
{
}

impl<G: Pinned<T>, T, Other> SameSize<Other> for Unpinned<G, T> {}

/// Holds when `Self` and `Other` are pinned layouts of the same size and
/// alignment. `#[typestate]` requires it without `align = N`.
#[doc(hidden)]
#[diagnostic::on_unimplemented(message = "convert with `morph_with` \
                                          instead of transmuting: \
                                          `{Other}` and `{Self}` differ \
                                          in size")]
pub trait SameLayout<Other> {}

impl<G1, T1, G2, T2, const SIZE: usize, const A1: usize, const A2: usize>
    SameLayout<Layout<G2, T2, SIZE, A2>> for Layout<G1, T1, SIZE, A1>
where
    Aligned<G1, T1, A1>: SameAlign<Aligned<G2, T2, A2>>,
{
}

impl<G1, T1, G2: Pinned<T2>, T2, const SIZE: usize, const ALIGN: usize>
    SameLayout<Unpinned<G2, T2>> for Layout<G1, T1, SIZE, ALIGN>
{
}

impl<G: Pinned<T>, T, Other> SameLayout<Other> for Unpinned<G, T> {}

/// Holds when `Self` and `Other` have the same alignment. Checked only
/// once [`SameLayout`] matched the sizes.
#[doc(hidden)]
#[diagnostic::on_unimplemented(message = "add `align = N` to \
                                          `#[typestate(..)]`, with `N` \
                                          at least the larger alignment \
                                          of `{Other}` and `{Self}`")]
pub trait SameAlign<Other> {}

impl<G1, T1, G2, T2, const ALIGN: usize> SameAlign<Aligned<G2, T2, ALIGN>>
    for Aligned<G1, T1, ALIGN>
{
}

/// Converts one projection for `morph`, through the morpher `M` from
/// state `Src` to `Dst`.
///
/// `Self` is the key type `#[template]` generates per associated type, so
/// the template's crate can implement it for every `M`.
#[doc(hidden)]
pub trait MorphLeaf<M, Src, Dst> {
    type In;
    type Out;

    fn morph_leaf(m: &mut M, value: Self::In) -> Self::Out;
}

/// Marks that `Self` can have the [`State`] `To` plugged in, in place of
/// its own, without changing its layout.
///
/// # Safety
///
/// `Self` and `Target` must be the same container with the same field
/// offsets, size and alignment.
#[diagnostic::on_unimplemented(
    message = "add `unsafe_transmute = true` to the `#[typestate]` of \
               `{Self}` to transmute it into state `{To}`",
    note = "both states' groups need the same `#[size(N)]`, and the \
            target must be the same struct",
    note = "or convert by value with `morph_with`, which needs neither"
)]
pub unsafe trait TransmutableState<To: State>:
    WithState + Sized
{
    /// `Self` with `To` as its state and every other generic unchanged.
    type Target: WithState<State = To>;

    /// Compile-time check that `Self` and [`Target`](Self::Target) share
    /// a size and alignment. `#[typestate]` overrides it to also compare
    /// every field's offset.
    ///
    /// ```ignore
    /// const _: () = <Wrap<Small> as TransmutableState<Big>>::LAYOUT_CHECK;
    /// ```
    const LAYOUT_CHECK: () = {
        assert!(
            core::mem::size_of::<Self>()
                == core::mem::size_of::<Self::Target>(),
            "`Self` and `Target` must have the same size"
        );
        assert!(
            core::mem::align_of::<Self>()
                == core::mem::align_of::<Self::Target>(),
            "`Self` and `Target` must have the same alignment"
        );
    };
}

/// Bit-reinterpretation transitions between [`State`]s of the same
/// [`WithState`] container type.
///
/// Each method takes the target state and returns
/// [`TransmutableState::Target`], `Self` with that state swapped in:
///
/// ```ignore
/// #[typestate(state = S, unsafe_transmute = true)]
/// struct Wrap<S: Meta> { value: S::Value }
///
/// let big = unsafe { small.transmute_state::<Big>() }; // Wrap<Small> -> Wrap<Big>
/// ```
///
/// `#[typestate(unsafe_transmute = true)]` implements
/// [`TransmutableState`] only for target states whose projections share
/// the source's `#[size(N)]` and alignment.
pub trait Isomorphic: WithState + Sized {
    /// Bit-reinterprets `self` as the same container type with the
    /// [`State`] `To` plugged in.
    ///
    /// # Safety
    ///
    /// The caller must ensure the bit pattern of `Self` is a valid
    /// instance of the target at every byte the two types share.
    ///
    /// Size equality is guaranteed by the [`TransmutableState`] bound,
    /// and rechecked by [`TransmutableState::LAYOUT_CHECK`].
    unsafe fn transmute_state<To: State>(self) -> Self::Target
    where
        Self: TransmutableState<To>,
    {
        const { Self::LAYOUT_CHECK }
        let value = core::mem::ManuallyDrop::new(self);
        // SAFETY: `TransmutableState` guarantees `Self` and `Target` share
        // a layout, and `ManuallyDrop` keeps `self` from being dropped
        // twice.
        unsafe { core::mem::transmute_copy::<Self, Self::Target>(&*value) }
    }

    /// Bit-reinterprets `&self` as a reference to the same container type
    /// with the [`State`] `To` plugged in.
    ///
    /// # Safety
    ///
    /// The caller must ensure the bit pattern of `Self` is a valid
    /// instance of the target at every byte the two types share.
    ///
    /// Size and alignment equality is guaranteed by the
    /// [`TransmutableState`] bound, and rechecked by
    /// [`TransmutableState::LAYOUT_CHECK`].
    unsafe fn transmute_state_ref<To: State>(&self) -> &Self::Target
    where
        Self: TransmutableState<To>,
    {
        const { Self::LAYOUT_CHECK }
        // SAFETY: `TransmutableState` guarantees `Self` and `Target` share
        // a layout, and the borrow keeps `self`'s lifetime.
        unsafe { &*(self as *const Self as *const Self::Target) }
    }

    /// Bit-reinterprets `&mut self` as a mutable reference to the same
    /// container type with the [`State`] `To` plugged in.
    ///
    /// # Safety
    ///
    /// The caller must ensure the bit pattern of `Self` is a valid
    /// instance of the target at every byte the two types share.
    ///
    /// Size and alignment equality is guaranteed by the
    /// [`TransmutableState`] bound, and rechecked by
    /// [`TransmutableState::LAYOUT_CHECK`].
    unsafe fn transmute_state_mut<To: State>(
        &mut self,
    ) -> &mut Self::Target
    where
        Self: TransmutableState<To>,
    {
        const { Self::LAYOUT_CHECK }
        // SAFETY: `TransmutableState` guarantees `Self` and `Target` share
        // a layout, and the borrow keeps `self`'s lifetime and uniqueness.
        unsafe { &mut *(self as *mut Self as *mut Self::Target) }
    }
}

impl<T: WithState> Isomorphic for T {}
