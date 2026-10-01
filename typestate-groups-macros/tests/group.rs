#![allow(dead_code)]

use typestate_groups_macros::{group, state};

trait Testing {
    type Meta;
    type AnotherType: Sized;
    type __TypestateGroupsLayoutMeta;
    type __TypestateGroupsLayoutAnotherType;
    type Marker: TestingGroupMarker<
            Meta = Self::Meta,
            AnotherType = Self::AnotherType,
        >;
}

trait TestingGroupMarker {
    type Meta;
    type AnotherType: Sized;
}

#[state]
pub struct StateA;
#[state]
pub struct StateB;

#[group(TestGroup)]
impl Testing for (StateA, StateB) {
    type Meta = usize;
    type AnotherType = u64;
}
