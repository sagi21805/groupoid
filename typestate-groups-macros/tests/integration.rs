use typestate_groups_macros::{group, state, state_types, typestate};

#[state_types]
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

#[typestate(state = T)]
struct Example<T: Testing> {
    _meta: T::Meta,
}

#[typestate_groups_macros::group_trait(by = Testing)]
trait A {
    fn a(&self);

    fn b(&self) {}
}

#[typestate_groups_macros::group(AnotherGroup)]
impl Testing for (StateC, StateD) {
    type Meta = u64;
}

#[typestate_groups_macros::group_impl(TestGroup)]
impl<S: Testing> A for Example<S> {
    fn a(&self) {
        eprintln!("TestGroup implementation!");
    }

    fn b(&self) {
        eprintln!("TestingGroup b implemenetation")
    }
}

#[typestate_groups_macros::group_impl(AnotherGroup)]
impl<S: Testing> A for Example<S> {
    fn a(&self) {
        eprintln!("AnotherGroup implementation!");
    }

    fn b(&self) {
        eprintln!("AnotherGroup b implemenetation")
    }
}

#[test]
fn dispatches_to_group_impl() {
    let example: Example<StateA> = Example { _meta: 3 };
    example.b();
    example.a();
    let example: Example<StateD> = Example { _meta: 3 };
    example.b();
}
