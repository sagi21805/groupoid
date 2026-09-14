use groupoid_macros::{blueprint, group, group_impl, group_trait, state, typestate};

#[blueprint]
trait Testing {
    type Meta;
}

#[state]
struct StateA;
#[state]
struct StateB;

#[group(GroupOne)]
impl Testing for (StateA,) {
    type Meta = u8;
}

#[group(GroupTwo)]
impl Testing for (StateB,) {
    type Meta = u16;
}

#[typestate(state = S)]
struct Widget<S: Testing> {
    #[allow(dead_code)]
    meta: S::Meta,
}

#[group_trait(by = Testing)]
trait Fetch {
    async fn fetch(&self, id: u32) -> u32;

    async fn combine(&self, a: u32, b: u32) -> u32;
}

#[group_impl(GroupOne)]
impl<S: Testing + groupoid::State> Fetch for Widget<S> {
    async fn fetch(&self, id: u32) -> u32 {
        id + 1
    }

    async fn combine(&self, a: u32, b: u32) -> u32 {
        a + b
    }
}

#[group_impl(GroupTwo)]
impl<S: Testing + groupoid::State> Fetch for Widget<S> {
    async fn fetch(&self, id: u32) -> u32 {
        id * 2
    }

    async fn combine(&self, a: u32, b: u32) -> u32 {
        a * b
    }
}

#[test]
fn async_method_forwards_through_await_and_dispatches_per_group() {
    let a = Widget::<StateA> { meta: 1 };
    let b = Widget::<StateB> { meta: 1 };
    assert_eq!(pollster::block_on(a.fetch(4)), 5);
    assert_eq!(pollster::block_on(b.fetch(4)), 8);
}

#[test]
fn async_method_with_multiple_forwarded_args_dispatches_per_group() {
    let a = Widget::<StateA> { meta: 1 };
    let b = Widget::<StateB> { meta: 1 };
    assert_eq!(pollster::block_on(a.combine(3, 4)), 7);
    assert_eq!(pollster::block_on(b.combine(3, 4)), 12);
}
