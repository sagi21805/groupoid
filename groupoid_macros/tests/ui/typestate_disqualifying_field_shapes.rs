// Field shapes that mention the state without being a bare `S::Assoc`
// projection disqualify a struct from `unsafe_transmute = true`, because its
// layout stops being a function of the projection's size and alignment alone:
//
// - `[S::Value; 2]` scales with the projection but is not pinned by it;
// - a bare `S` is the state marker itself, which nothing pins;
// - `<S as Meta>::Value` is the same type as `S::Value` but written with a
//   qualified path, which the deliberately narrow projection check rejects
//   rather than trying to normalise.
//
// Each is reported at the `#[typestate]` site, against the offending field
// type. Without the flag the same structs are accepted as they are and still
// get `restate` / `restate_with`.
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
