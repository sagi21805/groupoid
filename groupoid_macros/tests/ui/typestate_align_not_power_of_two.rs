// `align = N` is forwarded verbatim into `#[repr(align(N))]`, so rustc's own
// rules - a power of two no larger than 2^29 - apply without the macro
// re-checking them. The literal keeps the caller's span through the
// expansion, so E0589 still points at the `align = ..` they wrote.
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

#[typestate(state = S, unsafe_transmute = true, align = 3)]
struct NotPowerOfTwo<S: Meta> {
    value: S::Value,
}

#[typestate(state = S, unsafe_transmute = true, align = 1073741824)]
struct TooLarge<S: Meta> {
    value: S::Value,
}

fn main() {}
