// Documents a real limitation: #[group] splices its entire impl body
// verbatim into the generated `impl {Trait}GroupMarker for GroupName { .. }`,
// but the marker trait (from #[blueprint]) only ever declares associated
// *types*. Overriding a fn/const from the blueprint trait inside a #[group]
// body therefore breaks the marker impl with a real rustc error (the fn/const
// is "not a member of trait").
#![allow(dead_code)]
use groupoid_macros::{blueprint, group, state};

#[blueprint]
trait Testing {
    type Meta;

    fn f(&self);
}

#[state]
struct StateA;

#[group(G)]
impl Testing for (StateA,) {
    type Meta = u8;

    fn f(&self) {}
}

fn main() {}
