#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod restate;

pub use groupoid_macros::*;
pub use restate::Restate;

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

// TODO: possible bugs may occur because of this, should ensure that the
// SIZE and the ALIGN are matching.

/// A group whose type has a known compile-time size.
#[diagnostic::on_unimplemented(message = "add `#[size({SIZE})]` to the \
                                          associated type in \
                                          `#[group({Self})]`")]
pub trait SizedGroup<const SIZE: usize>: Group {}

/// A group whose type has a known compile-time alignment.
#[diagnostic::on_unimplemented(
    message = "add `align = N` to `#[typestate(..)]` to transmute into a \
               state of `{Self}`",
    note = "the value of `{Self}` is not {ALIGN}-byte aligned; `N` must \
            be at least the largest alignment among the states"
)]
pub trait AlignedGroup<const ALIGN: usize>: Group {}

/// A type that contains a group state, which implements [`SizedGroup`] and
/// [`AlignedGroup`].
pub trait SizedWithState<const SIZE: usize, const ALIGN: usize>:
    WithState + Sized
{
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
    note = "or convert by value with `restate_with`, which needs neither"
)]
pub unsafe trait TransmutableState<
    To: State,
    const SIZE: usize,
    const ALIGN: usize,
>: SizedWithState<SIZE, ALIGN>
{
    /// `Self` with `To` as its state and every other generic unchanged.
    type Target: SizedWithState<SIZE, ALIGN> + WithState<State = To>;

    /// Compile-time check that `Self` and [`Target`](Self::Target) share
    /// a size and alignment.
    ///
    /// ```ignore
    /// const _: () = <Wrap<Small> as TransmutableState<Big, 4, 4>>::LAYOUT_CHECK;
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
/// Both states must agree on `SIZE` and `ALIGN` (see [`SizedWithState`]).
pub trait TransmuteState<const SIZE: usize, const ALIGN: usize>:
    WithState + Sized
{
    /// Bit-reinterprets `self` as the same container type with the
    /// [`State`] `To` plugged in.
    ///
    /// # Safety
    ///
    /// The caller must ensure the bit pattern of `Self` is a valid
    /// instance of the target at every byte the two types share.
    ///
    /// Size equality is guaranteed by the [`SizedWithState`] bound, and
    /// rechecked by [`TransmutableState::LAYOUT_CHECK`].
    unsafe fn transmute_state<To: State>(self) -> Self::Target
    where
        Self: TransmutableState<To, SIZE, ALIGN>,
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
    /// Size and Alignment equality is guaranteed by the [`SizedWithState`]
    /// bound, and rechecked by [`TransmutableState::LAYOUT_CHECK`].
    unsafe fn transmute_state_ref<To: State>(&self) -> &Self::Target
    where
        Self: TransmutableState<To, SIZE, ALIGN>,
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
    /// Size and Alignment equality is guaranteed by the [`SizedWithState`]
    /// bound, and rechecked by [`TransmutableState::LAYOUT_CHECK`].
    unsafe fn transmute_state_mut<To: State>(
        &mut self,
    ) -> &mut Self::Target
    where
        Self: TransmutableState<To, SIZE, ALIGN>,
    {
        const { Self::LAYOUT_CHECK }
        // SAFETY: `TransmutableState` guarantees `Self` and `Target` share
        // a layout, and the borrow keeps `self`'s lifetime and uniqueness.
        unsafe { &mut *(self as *mut Self as *mut Self::Target) }
    }
}

impl<T: WithState, const SIZE: usize, const ALIGN: usize>
    TransmuteState<SIZE, ALIGN> for T
{
}
