use groupoid::group;

trait Testing {
    type Meta;
    type AnotherType: Sized;
    type Marker: TestingGroupMarker<Meta = Self::Meta, AnotherType = Self::AnotherType>;
}

trait TestingGroupMarker {
    type Meta;
    type AnotherType: Sized;
}

pub struct StateA;
pub struct StateB;

#[group(TestGroup)]
impl Testing for (StateA, StateB) {
    type Meta = usize;
    type AnotherType = u64;
}
