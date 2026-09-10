use groupoid_macros::{blueprint, group, state, typestate};

#[blueprint]
trait Testing {
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

#[typestate(state = T)]
struct Example<T: Testing> {
    meta: T::Meta,
}

#[groupoid_macros::group_trait(by = Testing)]
trait A {
    fn a(&self);
}

#[groupoid_macros::group_impl(TestGroup)]
impl<S: Testing> ExampleTrait for Example<S> {
    fn testing(test: usize) {}
}
