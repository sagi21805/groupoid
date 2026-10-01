//! A `pub` `#[state_types]` trait exposes its marker trait and group
//! structs.

mod states {
    use typestate_groups_macros::{group, state, state_types};

    #[state_types]
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
fn group_of_public_state_types_is_public() {
    let value: <states::Small as states::Meta>::Value = 3;
    let _: states::Numbers = states::Numbers;
    assert_eq!(value, 3);
}
