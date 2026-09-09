use groupoid_macros::{blueprint, group};

#[blueprint]
trait Testing {
    type Meta;
    type AnotherType: Sized;
}

pub struct StateA;
pub struct StateB;

#[group(TestGroup)]
impl Testing for (StateA, StateB) {
    type Meta = usize;
    type AnotherType = u64;
}

struct Example<T: Testing> {
    meta: T::Meta,
}

pub trait HasState {
    type State;
}

impl<T: Testing> HasState for Example<T> {
    type State = T;
}

#[groupoid_macros::group_trait(by = Testing)]
trait A {
    fn a(&self);
}

#[groupoid_macros::group_impl(TestGroup)]
impl<S: Testing> ExampleTrait for Example<S> {
    fn testing(test: usize) {}
}
