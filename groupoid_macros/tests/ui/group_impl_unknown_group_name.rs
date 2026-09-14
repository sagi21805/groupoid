// Documents current (unfriendly) behavior: #[group_impl] does not check that
// its group name was ever produced by #[group]. Referencing a name that was
// never declared surfaces as a plain "cannot find type" error from rustc.
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
trait Fetch {
    fn fetch(&self) -> u32;
}

#[group_impl(NeverDeclared)]
impl<S: Testing> Fetch for Widget<S> {
    fn fetch(&self) -> u32 {
        1
    }
}

fn main() {}
