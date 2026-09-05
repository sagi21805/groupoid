use groupoid::{blueprint, group_impl};

#[blueprint]
trait Testing {
    type Meta;
    type AnotherType: Sized;
}

pub struct StateA;
pub struct StateB;

#[group_impl(TestGroup)]
impl Testing for (StateA, StateB) {
    type Meta = usize;
    type AnotherType = u64;
}

struct Example<T: Testing> {
    meta: T::Meta,
}

trait ExampleTrait {
    fn testing(test: usize) {}
}
