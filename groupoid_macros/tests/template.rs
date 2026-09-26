#![allow(dead_code)]

use groupoid_macros::{group, state, template};

#[template]
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
fn group_implements_template_methods() {
    assert_eq!(StateA::some_function(), 7);
}
