#![allow(dead_code)]

use typestate_groups_macros::{group, state, state_types};

#[state_types]
trait Testing {
    type Meta;

    fn some_function() -> usize;

    const TESTING: usize = 3;
}

#[state]
struct StateA;

#[group(GroupA)]
impl Testing for (StateA,) {
    type Meta = u8;

    fn some_function() -> usize {
        7
    }
}

#[test]
fn group_implements_state_types_methods() {
    assert_eq!(StateA::some_function(), 7);
}
