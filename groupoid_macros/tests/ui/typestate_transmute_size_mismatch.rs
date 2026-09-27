// States whose projections differ in `#[size(N)]` don't transmute.
#![allow(dead_code)]
use groupoid::Isomorphic;
use groupoid_macros::{group, state, template, typestate};

#[template]
trait Meta {
    type Value;
}

#[state]
struct Small;
#[state]
struct Big;

#[group(SmallGroup)]
impl Meta for (Small,) {
    #[size(1)]
    type Value = u8;
}

#[group(BigGroup)]
impl Meta for (Big,) {
    #[size(8)]
    type Value = u64;
}

#[typestate(state = S, unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Value,
}

fn main() {
    let small = Wrap::<Small> { value: 0 };
    let _big = unsafe { small.transmute_state::<Big>() };
}
