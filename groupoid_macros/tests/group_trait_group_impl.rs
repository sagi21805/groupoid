#![allow(non_camel_case_types)]

use groupoid_macros::{
    blueprint, group, group_impl, group_trait, state, typestate,
};

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

// Deliberately underscore/snake_case trait name, plus a const, a multi-arg
// method, and a method with a default body.
#[group_trait(by = Testing)]
trait a_b {
    const LIMIT: u32 = 10;

    fn sum_three(&self, x: i32, y: i32, z: i32) -> i32;

    fn tagged(&self) -> &'static str {
        "default_tag"
    }
}

#[group_impl(GroupOne)]
impl<S: Testing + groupoid::State> a_b for Widget<S> {
    const LIMIT: u32 = 42; // only reaches the private helper-trait impl.

    fn sum_three(&self, x: i32, y: i32, z: i32) -> i32 {
        x + y + z
    }

    fn tagged(&self) -> &'static str {
        "group_one"
    }
}

#[group_impl(GroupTwo)]
impl<S: Testing + groupoid::State> a_b for Widget<S> {
    const LIMIT: u32 = 99; // only reaches the private helper-trait impl.

    fn sum_three(&self, x: i32, y: i32, z: i32) -> i32 {
        x * y * z
    }

    fn tagged(&self) -> &'static str {
        "group_two"
    }
}

#[test]
fn forwards_multiple_positional_args_and_dispatches_correctly() {
    let a = Widget::<StateA> { meta: 1 };
    let b = Widget::<StateB> { meta: 1 };
    assert_eq!(a.sum_three(1, 2, 3), 6);
    assert_eq!(b.sum_three(2, 3, 4), 24);
}

#[test]
fn default_bodied_method_must_be_reimplemented_by_each_group_impl() {
    let a = Widget::<StateA> { meta: 1 };
    let b = Widget::<StateB> { meta: 1 };
    assert_eq!(a.tagged(), "group_one");
    assert_eq!(b.tagged(), "group_two");
}

#[test]
fn associated_const_is_not_delegated_and_always_uses_trait_level_default()
{
    // The public blanket impl never forwards consts, so both states see
    // the trait-level default...
    assert_eq!(<Widget<StateA> as a_b>::LIMIT, 10);
    assert_eq!(<Widget<StateB> as a_b>::LIMIT, 10);

    // ...even though each group's override *did* land on the private
    // helper trait, reachable directly from within this same crate
    // root.
    assert_eq!(
        <Widget<StateA> as __a_b_helper_mod::a_bHelper<GroupOne>>::LIMIT,
        42
    );
    assert_eq!(
        <Widget<StateB> as __a_b_helper_mod::a_bHelper<GroupTwo>>::LIMIT,
        99
    );
}

// A second group_trait with different ident casing: CamelCase trait name,
// snake_case method name.
#[group_trait(by = Testing)]
trait MyCoolTrait {
    fn do_the_thing(&self) -> &'static str;
}

#[group_impl(GroupOne)]
impl<S: Testing + groupoid::State> MyCoolTrait for Widget<S> {
    fn do_the_thing(&self) -> &'static str {
        "one"
    }
}

#[group_impl(GroupTwo)]
impl<S: Testing + groupoid::State> MyCoolTrait for Widget<S> {
    fn do_the_thing(&self) -> &'static str {
        "two"
    }
}

#[test]
fn camel_case_trait_and_snake_case_method_idents_dispatch_correctly() {
    let a = Widget::<StateA> { meta: 1 };
    let b = Widget::<StateB> { meta: 1 };
    assert_eq!(a.do_the_thing(), "one");
    assert_eq!(b.do_the_thing(), "two");
}
