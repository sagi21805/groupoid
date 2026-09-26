use groupoid_macros::{
    group, group_impl, group_trait, state, template, typestate,
};

#[template]
trait Testing {
    type Meta;
}

#[state]
pub struct StateA;
#[state]
pub struct StateB;
#[state]
pub struct StateC;
#[state]
pub struct StateD;

#[group(TestGroup)]
impl Testing for (StateA, StateB) {
    type Meta = usize;
}

#[group(AnotherGroup)]
impl Testing for (StateC, StateD) {
    type Meta = u64;
}

#[typestate(state = T)]
struct Example<T: Testing> {
    #[allow(dead_code)]
    meta: T::Meta,
}

#[group_trait(by = Testing)]
trait A {
    fn a(&self) -> &'static str;

    fn b(&self) -> &'static str {
        "default b"
    }
}

#[group_impl(TestGroup)]
impl<S: Testing + groupoid::State> A for Example<S> {
    fn a(&self) -> &'static str {
        "TestGroup::a"
    }

    fn b(&self) -> &'static str {
        "TestGroup::b"
    }
}

#[group_impl(AnotherGroup)]
impl<S: Testing + groupoid::State> A for Example<S> {
    fn a(&self) -> &'static str {
        "AnotherGroup::a"
    }

    fn b(&self) -> &'static str {
        "AnotherGroup::b"
    }
}

#[test]
fn every_state_in_test_group_dispatches_to_test_group_impl() {
    let example: Example<StateA> = Example { meta: 3 };
    assert_eq!(example.a(), "TestGroup::a");
    assert_eq!(example.b(), "TestGroup::b");

    let example: Example<StateB> = Example { meta: 3 };
    assert_eq!(example.a(), "TestGroup::a");
    assert_eq!(example.b(), "TestGroup::b");
}

#[test]
fn every_state_in_another_group_dispatches_to_another_group_impl() {
    let example: Example<StateC> = Example { meta: 3 };
    assert_eq!(example.a(), "AnotherGroup::a");
    assert_eq!(example.b(), "AnotherGroup::b");

    let example: Example<StateD> = Example { meta: 3 };
    assert_eq!(example.a(), "AnotherGroup::a");
    assert_eq!(example.b(), "AnotherGroup::b");
}
