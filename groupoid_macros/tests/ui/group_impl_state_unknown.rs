// `state` must name one of the impl's type parameters.
#![allow(dead_code)]
use groupoid_macros::{group, group_impl, group_trait, state, template, typestate};

#[template]
trait Meta {
    type Value;
}

#[state]
struct Small;

#[group(Numbers)]
impl Meta for (Small,) {
    type Value = u32;
}

#[typestate(state = S)]
struct Wrap<S: Meta, I> {
    value: S::Value,
    extra: I,
}

#[group_trait(by = Meta)]
trait Describe {
    fn describe(&self) -> String;
}

#[group_impl(Numbers, state = T)]
impl<S: Meta, I> Describe for Wrap<S, I> {
    fn describe(&self) -> String {
        String::new()
    }
}

fn main() {}
