// `#[group_impl]` sets the `#[state_types]` trait's associated type from the group, so
// an `Assoc = Type` bound on the impl is rejected.
#![allow(dead_code)]
use typestate_groups_macros::{group, group_impl, group_trait, state, state_types, typestate};

#[state_types]
trait Meta {
    type Value;
}

#[state]
struct Small;

#[group(Numbers)]
impl Meta for (Small,) {
    type Value = u32;
}

#[typestate]
struct Wrap<S: Meta> {
    value: S::Value,
}

#[group_trait(by = Meta)]
trait Describe {
    fn describe(&self) -> String;
}

#[group_impl(Numbers)]
impl<S: Meta<Value = String>> Describe for Wrap<S> {
    fn describe(&self) -> String {
        self.value.to_uppercase()
    }
}

fn main() {}
