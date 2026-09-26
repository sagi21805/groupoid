// `#[group_trait]` rejects an associated type, which the blanket impl
// couldn't define.
#![allow(dead_code)]
use groupoid_macros::{group_trait, template};

#[template]
trait Testing {
    type Meta;
}

#[group_trait(by = Testing)]
trait Foo {
    type Extra;

    fn a(&self) -> Self::Extra;
}

fn main() {}
