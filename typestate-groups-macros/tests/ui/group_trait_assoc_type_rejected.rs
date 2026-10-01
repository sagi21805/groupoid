// `#[group_trait]` rejects an associated type, which the blanket impl
// couldn't define.
#![allow(dead_code)]
use typestate_groups_macros::{group_trait, state_types};

#[state_types]
trait Testing {
    type Meta;
}

#[group_trait(by = Testing)]
trait Foo {
    type Extra;

    fn a(&self) -> Self::Extra;
}

fn main() {}
