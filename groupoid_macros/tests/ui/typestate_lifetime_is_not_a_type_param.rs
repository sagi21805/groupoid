// `'a` is a lifetime token, not a valid `Ident`, so this fails while parsing
// the `#[typestate(...)]` attribute grammar itself -- before the semantic
// "must be one of the generic type parameters" check ever runs.
#![allow(dead_code)]
use groupoid_macros::typestate;

#[typestate(state = 'a)]
struct Foo<'a> {
    s: &'a str,
}

fn main() {}
