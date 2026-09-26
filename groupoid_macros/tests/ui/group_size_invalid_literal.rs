// Documents that `#[size(..)]` requires a plain integer literal argument;
// anything else fails to parse and is reported as a macro-authored
// compile_error! rather than reaching the generated size assertion.
#![allow(dead_code)]
use groupoid_macros::{blueprint, group, state};

#[blueprint]
trait Testing {
    type Meta;
}

#[state]
struct StateA;

#[group(G)]
impl Testing for (StateA,) {
    #[size("eight")]
    type Meta = u64;
}

fn main() {}
