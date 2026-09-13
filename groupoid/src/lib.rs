pub trait State {}

pub trait WithState {
    type State: State;
}

pub trait Group {}

pub trait GroupMarker {
    type Marker: Group;
}

/// A group that it's type has a known compiletime size.
pub trait SizedGroup<const SIZE: usize>: Group {}

/// A type that contains a group state, which implements [`SizedGroup`].
///
/// The size contains the compile time known size of the state type.
pub trait SizedWithState<const SIZE: usize>: WithState + Sized {}

/// Marks that `Self` and `To` are the exact same [`WithState`] container
/// type, differing only by [`State`], ensuring that both states have a type
/// with the same size.
///
/// [`typestate`] generates this once per struct, as a single impl generic
/// over *every* other state that could be plugged in.
///
/// This trait should never be written by hand, and it should never hold between
/// two structurally unrelated `WithState` types that may happen to share
/// a size. Like [`SizedWithState`], it only exists for structs who has a fields
/// that is state dependent
///
/// # Safety
///
/// Implementing this trait is an assertion that `Self` and `To` are actually
/// the same underlying container with the same state value size.
pub unsafe trait TransmutableState<To: SizedWithState<SIZE>, const SIZE: usize>:
    SizedWithState<SIZE>
{
}

pub use groupoid_macros::*;

/// Bit-reinterpretation transitions between [`State`]s of the same
/// [`WithState`] container type.
pub trait TransmuteState: WithState + Sized {
    /// Bit-reinterprets `self` as the same container type with a different
    /// [`State`] plugged in.
    ///
    /// # Safety
    ///
    /// The caller must ensure the bit pattern of `Self` is a valid instance
    /// of `To` at every byte the two types share.
    ///
    /// Size equality is guaranteed by the SizeWithState<SIZE> trait.
    unsafe fn transmute_state<To: SizedWithState<SIZE>, const SIZE: usize>(self) -> To
    where
        Self: TransmutableState<To, SIZE>,
    {
        let value = core::mem::ManuallyDrop::new(self);
        // SAFETY: `Self` and `To` are the same struct with their state's
        // projected value pinned to the same size on both sides
        // `TransmutableState`
        unsafe { core::mem::transmute_copy::<Self, To>(&*value) }
    }

    /// Bit-reinterprets `&self` as a `&To`. See [`transmute_state`] for the
    /// [`TransmutableState`] bound this relies on.
    ///
    /// # Safety
    /// Both types must have the save alignment, so thier pointers could cast
    /// into one another.
    unsafe fn transmute_state_ref<To: SizedWithState<SIZE>, const SIZE: usize>(&self) -> &To
    where
        Self: TransmutableState<To, SIZE>,
    {
        const {
            assert!(
                core::mem::align_of::<To>() <= core::mem::align_of::<Self>(),
                "TransmuteState::transmute_state_ref: `To`'s alignment must not exceed `Self`'s"
            );
        }
        unsafe { &*(self as *const Self as *const To) }
    }

    /// Bit-reinterprets `&mut self` as a `&mut To`. See [`transmute_state`]
    /// for the [`TransmutableState`] bound this relies on.
    ///
    /// # Safety
    ///
    /// Both types must have the save alignment, so thier pointers could cast
    /// into one another.
    unsafe fn transmute_state_mut<To: SizedWithState<SIZE>, const SIZE: usize>(&mut self) -> &mut To
    where
        Self: TransmutableState<To, SIZE>,
    {
        const {
            assert!(
                core::mem::align_of::<To>() <= core::mem::align_of::<Self>(),
                "TransmuteState::transmute_state_mut: `To`'s alignment must not exceed `Self`'s"
            );
        }
        unsafe { &mut *(self as *mut Self as *mut To) }
    }
}

impl<T: WithState> TransmuteState for T {}
