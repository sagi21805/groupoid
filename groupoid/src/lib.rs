pub trait State {}

pub trait WithState {
    type State: State;
}

pub trait Group {}

pub trait GroupMarker {
    type Marker: Group;
}

pub use groupoid_macros::*;
