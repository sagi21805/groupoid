use groupoid_macros::{
    group, group_impl, group_trait, state, template, typestate,
};

#[template]
trait Meta {
    type Value;
}

#[state]
struct Small;

#[group(Numbers)]
impl Meta for (Small,) {
    type Value = u32;
}

#[typestate(state = S)]
struct Wrap<S: Meta, I> {
    value: S::Value,
    extra: I,
}

#[group_trait(by = Meta)]
trait Describe {
    fn describe(&self) -> u32;
}

// The `Item = u32` bindings bound `I`, not the state, so `#[group_impl]`
// keeps them, inline and in the `where` clause.
#[group_impl(Numbers, state = S)]
impl<S: Meta, I: Iterator<Item = u32> + Clone> Describe for Wrap<S, I>
where
    I: ExactSizeIterator<Item = u32>,
{
    fn describe(&self) -> u32 {
        self.value + self.extra.clone().sum::<u32>()
    }
}

#[test]
fn group_impl_keeps_bindings_on_other_parameters() {
    let wrap = Wrap::<Small, _> {
        value: 1,
        extra: [2, 3].into_iter(),
    };
    assert_eq!(wrap.describe(), 6);
}
