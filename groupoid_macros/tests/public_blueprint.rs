//! A `pub` blueprint trait exposes its marker trait and group structs.

mod states {
    use groupoid_macros::{blueprint, group, state};

    #[blueprint]
    pub trait Meta {
        type Value;
    }

    #[state]
    pub struct Small;

    #[group(Numbers)]
    impl Meta for (Small,) {
        type Value = u8;
    }
}

#[test]
fn group_of_public_blueprint_is_public() {
    let value: <states::Small as states::Meta>::Value = 3;
    let _: states::Numbers = states::Numbers;
    assert_eq!(value, 3);
}
