use groupoid::{blueprint, group};

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

trait ExampleTrait {
    fn testing(test: usize);
}

#[groupoid::group_impl(TestGroup)]
impl<S: Testing> ExampleTrait for Example<S> {
    fn testing(test: usize) {}
}
