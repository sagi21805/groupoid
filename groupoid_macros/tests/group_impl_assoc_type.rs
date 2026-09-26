use groupoid_macros::{
    group, group_impl, group_trait, state, template, typestate,
};

#[template]
trait Meta {
    type Value;
}

#[state]
struct Small;
#[state]
struct Big;

#[group(Numbers)]
impl Meta for (Small,) {
    type Value = u32;
}

#[group(Words)]
impl Meta for (Big,) {
    type Value = String;
}

#[typestate]
struct Wrap<S: Meta> {
    value: S::Value,
}

#[group_trait(by = Meta)]
trait Describe {
    fn describe(&self) -> String;
}

// Only `S: Meta`: `#[group_impl]` pins `S::Value` to the group's type.
#[group_impl(Numbers)]
impl<S: Meta> Describe for Wrap<S> {
    fn describe(&self) -> String {
        (self.value + 1).to_string()
    }
}

#[group_impl(Words)]
impl<S: Meta> Describe for Wrap<S> {
    fn describe(&self) -> String {
        self.value.to_uppercase()
    }
}

#[test]
fn group_impl_sees_the_groups_concrete_type() {
    assert_eq!(Wrap::<Small> { value: 1 }.describe(), "2");
    assert_eq!(Wrap::<Big> { value: "hi".into() }.describe(), "HI");
}
