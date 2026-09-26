// A layer with two different arguments that mention the state has no one
// inner type to restate, so the call site needs a whole-type impl.
use groupoid_macros::{blueprint, group, state, typestate};

#[blueprint]
trait Meta {
    type Value;
}

#[state]
struct Small;
#[state]
struct Big;

#[group(SmallGroup)]
impl Meta for (Small,) {
    type Value = u32;
}

#[group(BigGroup)]
impl Meta for (Big,) {
    type Value = i32;
}

#[typestate(state = S)]
struct Wrap<S: Meta> {
    value: Result<S::Value, [S::Value; 2]>,
}

fn main() {
    let small = Wrap::<Small> { value: Ok(1) };
    let _big: Wrap<Big> = small.restate_with(|v| v as i32);
}
