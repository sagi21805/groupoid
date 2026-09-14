// Documents current (unfriendly) behavior: #[group] does not check that its
// trait was ever declared with #[blueprint]. Omitting #[blueprint] means the
// `{Trait}GroupMarker` trait it references by naming convention was never
// generated, so this fails with a plain "cannot find trait" error from rustc
// rather than a diagnostic authored by the macro itself.
#![allow(dead_code)]
use groupoid_macros::{group, state};

trait Testing {
    type Meta;
}

#[state]
struct StateA;

#[group(G)]
impl Testing for (StateA,) {
    type Meta = u8;
}

fn main() {}
