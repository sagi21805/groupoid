//! `#[group]` sets a `#[state_types]` trait for every state in a tuple.

mod states {
    use typestate_groups_macros::{group, state, state_types};

    #[state_types]
    pub trait Meta {
        type Value;

        const VERSION: u32 = 1;

        fn id() -> usize;

        fn label() -> &'static str {
            "default"
        }
    }

    #[state]
    pub struct A;
    #[state]
    pub struct B;
    #[state]
    pub struct C;
    #[state]
    pub struct D;

    #[group(Solo)]
    impl Meta for (A,) {
        type Value = u8;

        fn id() -> usize {
            1
        }
    }

    #[group(Triple)]
    impl Meta for (B, C, D) {
        type Value = u16;

        fn id() -> usize {
            3
        }
    }

    #[group(Empty)]
    impl Meta for () {
        type Value = ();

        fn id() -> usize {
            0
        }
    }
}

use states::{A, B, C, D, Meta};

fn accepts_group<G: typestate_groups::Group>(_: G) {}

#[test]
fn group_sets_every_state_in_the_tuple() {
    let _: <A as Meta>::Value = 1u8;
    let _: (<B as Meta>::Value, <C as Meta>::Value, <D as Meta>::Value) =
        (1u16, 2u16, 3u16);

    assert_eq!((A::id(), B::id(), C::id(), D::id()), (1, 3, 3, 3));
    assert_eq!((A::label(), D::label()), ("default", "default"));
    assert_eq!(<C as Meta>::VERSION, 1);

    accepts_group(states::Solo);
    accepts_group(states::Empty);
}
