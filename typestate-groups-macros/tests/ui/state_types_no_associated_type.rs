// A `#[state_types]` trait needs an associated type for its groups to set.
#![allow(dead_code)]
use typestate_groups_macros::state_types;

#[state_types]
trait Meta {
    fn describe(&self) -> String;
}

fn main() {}
