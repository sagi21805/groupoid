// States of equal size but different alignment don't transmute without
// `align = N`.
#![allow(dead_code)]
use groupoid::Isomorphic;
use groupoid_macros::{group, state, template, typestate};

#[template]
trait Meta {
    type Value;
}

#[state]
struct Wide;
#[state]
struct Bytes;

#[group(WideGroup)]
impl Meta for (Wide,) {
    #[size(8)]
    type Value = u64;
}

#[group(BytesGroup)]
impl Meta for (Bytes,) {
    #[size(8)]
    type Value = [u8; 8];
}

#[typestate(state = S, unsafe_transmute = true)]
struct Wrap<S: Meta> {
    value: S::Value,
    tag: u8,
}

fn main() {
    let wide = Wrap::<Wide> { value: 0, tag: 0 };
    let _bytes = unsafe { wide.transmute_state::<Bytes>() };
}
