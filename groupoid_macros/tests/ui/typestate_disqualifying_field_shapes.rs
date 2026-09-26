// Fields that mention the state without being a bare `S::Assoc` reject
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
struct Arrayed<S: Meta> {
    values: [S::Value; 2],
}

#[typestate(state = S, unsafe_transmute = true)]
struct BareState<S: Meta> {
    value: S::Value,
    marker: S,
}

#[typestate(state = S, unsafe_transmute = true)]
struct Qualified<S: Meta> {
    value: <S as Meta>::Value,
}

fn main() {}
