// Documents the limit of `unsafe_transmute = true`: every field must be a bare
// `S::Assoc` projection, a `PhantomData`, or a type that never mentions the
// state. A projection wrapped in anything else cannot have its layout pinned,
// so the flag is rejected at the `#[typestate]` site rather than the traits
// silently going missing.
//
// `Option` is the sharpest case rather than an arbitrary one: niche
// optimisation can lay `Option<A>` and `Option<B>` out differently even when
// `A` and `B` agree on both size and alignment, so pinning the projection's
// layout would not pin the container's.
//
// A struct with *extra* fields is fine - see
// `tests/transmute_state_multi_field.rs` - as long as those fields do not
// mention the state. And without the flag, `Option<S::Value>` is perfectly
// restatable by value: see `tests/restate.rs`.
#![allow(dead_code)]
use groupoid_macros::{blueprint, group, state, typestate};

#[blueprint]
trait Meta {
    type Value;
}

#[state]
struct Small;

#[group(SmallGroup)]
impl Meta for (Small,) {
    #[size(1)]
    type Value = u8;
}

#[typestate(state = S, unsafe_transmute = true)]
struct NotSized<S: Meta> {
    value: Option<S::Value>,
}

fn main() {}
