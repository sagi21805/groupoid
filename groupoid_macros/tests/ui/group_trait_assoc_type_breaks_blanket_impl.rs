// Documents a real limitation: #[group_trait]'s generated blanket impl only
// ever delegates method (`TraitItem::Fn`) items. An associated *type*
// declared alongside methods on a #[group_trait] trait has no way to be
// satisfied on the blanket impl (stable Rust has no default associated
// types), so this breaks with a real "not all trait items implemented" error.
#![allow(dead_code)]
use groupoid_macros::{blueprint, group_trait};

#[blueprint]
trait Testing {
    type Meta;
}

#[group_trait(by = Testing)]
trait Foo {
    type Extra;

    fn a(&self) -> Self::Extra;
}

fn main() {}
