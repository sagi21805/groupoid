use groupoid_macros::{blueprint, group, state, typestate};
use syn::ExprArray;

#[blueprint]
trait Testing {
    type Meta;
    type AnotherType: Sized;
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
    type AnotherType = u64;
}

#[typestate(state = T)]
struct Example<T: Testing> {
    meta: T::Meta,
}

#[groupoid_macros::group_trait(by = Testing)]
trait A {
    fn a(&self);

    fn b(&self) {}
}

#[groupoid_macros::group(AnotherGroup)]
impl Testing for (StateC, StateD) {
    type Meta = u64;
    type AnotherType = u64;
}

#[groupoid_macros::group_impl(TestGroup)]
impl<S: Testing + groupoid::State> A for Example<S> {
    fn a(&self) {
        eprintln!("TestGroup implementation!");
    }

    fn b(&self) {
        eprintln!("TestingGroup b implemenetation")
    }
}

#[groupoid_macros::group_impl(AnotherGroup)]
impl<S: Testing + groupoid::State> A for Example<S> {
    fn a(&self) {
        eprintln!("AnotherGroup implementation!");
    }

    fn b(&self) {
        eprintln!("AnotherGroup b implemenetation")
    }
}

#[test]
fn dispatches_to_group_impl() {
    let example: Example<StateA> = Example { meta: 3 };
    example.b();
    example.a();
    let example: Example<StateD> = Example { meta: 3 };
    example.b();
}
