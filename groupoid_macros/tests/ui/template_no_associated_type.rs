// A template needs an associated type for its groups to set.
#![allow(dead_code)]
use groupoid_macros::template;

#[template]
trait Meta {
    fn describe(&self) -> String;
}

fn main() {}
