// A wrapper without a `Morph` impl fails where `morph_with` is
// called.
#![allow(dead_code)]
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
    #[size(4)]
    type Value = u32;
}

#[group(BigGroup)]
impl Meta for (Big,) {
    #[size(4)]
    type Value = i32;
}

struct Pair<T>(T, T);

#[typestate(state = S)]
struct Wrap<S: Meta> {
    user: Pair<S::Value>,
}

fn main() {
    let small = Wrap::<Small> { user: Pair(1, 2) };
    let _big: Wrap<Big> = small.morph_with(|v| v as i32);
}
