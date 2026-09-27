//! Bit-reinterpreting transitions between a container's states.

use crate::{State, WithState};

/// Bit-reinterprets a container as the same container in another
/// [`State`].
///
/// ```ignore
/// let big = unsafe { small.transmute_state::<Big>() }; // Wrap<Small> -> Wrap<Big>
/// ```
pub trait Isomorphic: WithState + Sized {
    /// Bit-reinterprets `self` in state `To`.
    ///
    /// # Safety
    ///
    /// The bits of `self` must be a valid instance of the target.
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

    /// Bit-reinterprets `&self` in state `To`.
    ///
    /// # Safety
    ///
    /// The bits of `self` must be a valid instance of the target.
    unsafe fn transmute_state_ref<To: State>(&self) -> &Self::Target
    where
        Self: TransmutableState<To>,
    {
        const { Self::LAYOUT_CHECK }
        // SAFETY: `TransmutableState` guarantees `Self` and `Target` share
        // a layout, and the borrow keeps `self`'s lifetime.
        unsafe { &*(self as *const Self as *const Self::Target) }
    }

    /// Bit-reinterprets `&mut self` in state `To`.
    ///
    /// # Safety
    ///
    /// The bits of `self` must be a valid instance of the target.
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

/// `Self` keeps its layout in state `To`.
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
