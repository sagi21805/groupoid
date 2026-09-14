use groupoid_macros::{blueprint, group, state};

#[blueprint]
trait Metadata {
    type Meta;

    const VERSION: u32 = 1;

    fn label() -> &'static str {
        "default"
    }
}

#[state]
struct StateA;
#[state]
struct StateB;
#[state]
struct StateC;
#[state]
struct StateD;

// A group of exactly one state.
#[group(SoloGroup)]
impl Metadata for (StateA,) {
    type Meta = u8;
}

// A group of three states.
#[group(TripleGroup)]
impl Metadata for (StateB, StateC, StateD) {
    type Meta = u16;
}

// A group of zero states.
#[group(EmptyGroup)]
impl Metadata for () {
    type Meta = ();
}

#[test]
fn solo_group_of_one_state_gets_marker_and_trait_defaults() {
    let _: <StateA as Metadata>::Meta = 5u8;
    assert_eq!(StateA::label(), "default");
    assert_eq!(<StateA as Metadata>::VERSION, 1);
}

#[test]
fn triple_group_all_three_states_get_identical_behavior() {
    let _: <StateB as Metadata>::Meta = 1u16;
    let _: <StateC as Metadata>::Meta = 2u16;
    let _: <StateD as Metadata>::Meta = 3u16;

    assert_eq!(StateB::label(), "default");
    assert_eq!(StateC::label(), "default");
    assert_eq!(StateD::label(), "default");

    assert_eq!(<StateB as Metadata>::VERSION, 1);
    assert_eq!(<StateC as Metadata>::VERSION, 1);
    assert_eq!(<StateD as Metadata>::VERSION, 1);
}

fn accepts_group<G: groupoid::Group>(_: G) {}

#[test]
fn empty_tuple_group_compiles_with_zero_per_state_impls() {
    accepts_group(EmptyGroup);
}
