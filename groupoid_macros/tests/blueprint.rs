use groupoid_macros::blueprint;

#[blueprint]
trait Testing {
    type Meta;
    type AnotherType: Sized;

    fn some_function();

    const TESTING: usize = 3;
}

fn main() {
    println!("Hello, world!");
}
