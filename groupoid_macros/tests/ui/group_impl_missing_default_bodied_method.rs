// Documents current behavior: #[group_trait] strips ALL method defaults when
// generating the private helper trait, so every #[group_impl] block must
// implement every method, even ones that had a default body on the original
// trait. Omitting one here breaks with a real "not all trait items
// implemented" error on the helper-trait impl.
#![allow(dead_code)]
use groupoid_macros::{blueprint, group, group_impl, group_trait, state, typestate};

#[blueprint]
trait Testing {
    type Meta;
}

#[state]
struct StateA;

#[group(GroupOne)]
impl Testing for (StateA,) {
    type Meta = u8;
}

#[typestate(state = S)]
struct Widget<S: Testing> {
    meta: S::Meta,
}

#[group_trait(by = Testing)]
trait Foo {
    fn a(&self);

    fn b(&self) -> &'static str {
        "default"
    }
}

#[group_impl(GroupOne)]
impl<S: Testing + groupoid::State> Foo for Widget<S> {
    fn a(&self) {}
    // Missing `fn b`, even though the original trait gave it a default body.
}

fn main() {}
