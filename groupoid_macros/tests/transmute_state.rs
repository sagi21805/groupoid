// `TransmuteState::transmute_state` (and its ref/mut forms) require the
// `TransmutableState` bound, which `#[typestate]` only ever generates between
// two states of the *same* struct whose blueprint values share a `SizedGroup`
// size. These tests exercise the accepted case; see
// `tests/ui/transmute_state_unrelated_types_rejected.rs` for the rejected one.
use groupoid::TransmuteState;
use groupoid_macros::{blueprint, group, state, typestate};

#[blueprint]
trait Meta {
    type Value;
}

#[state]
struct Small;
#[state]
struct Big;

#[group(SmallGroup)]
impl Meta for (Small,) {
    #[size(4)]
    type Value = u32;
}

#[group(BigGroup)]
impl Meta for (Big,) {
    #[size(4)]
    type Value = i32;
}

#[typestate(state = S)]
struct Wrap<S: Meta> {
    value: S::Value,
}

#[test]
fn transmute_state_between_same_sized_states_of_the_same_struct() {
    let small = Wrap::<Small> {
        value: 0xdead_beefu32,
    };
    let big: Wrap<Big> = unsafe { small.transmute_state() };
    assert_eq!(big.value, 0xdead_beefu32 as i32);
}

#[test]
fn transmute_state_ref_and_mut_round_trip() {
    let mut small = Wrap::<Small> { value: 7 };
    {
        let big_ref: &Wrap<Big> = unsafe { small.transmute_state_ref() };
        assert_eq!(big_ref.value, 7);
    }
    {
        let big_mut: &mut Wrap<Big> = unsafe { small.transmute_state_mut() };
        big_mut.value = 9;
    }
    assert_eq!(small.value, 9);
}

#[test]
fn extension_method_form_works() {
    let small = Wrap::<Small> { value: 3 };
    let big: Wrap<Big> = unsafe { small.transmute_state() };
    assert_eq!(big.value, 3);
}
