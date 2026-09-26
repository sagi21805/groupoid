// A wrapped projection such as `Option<S::Value>` rejects
// `unsafe_transmute = true`.
#![allow(dead_code)]
use groupoid_macros::{group, state, template, typestate};

#[template]
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
