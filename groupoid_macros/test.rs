#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use groupoid::Isomorphic;
use groupoid_macros::{group, state, template, typestate};
trait Meta: ::groupoid::State {
    type Value;
    #[doc(hidden)]
    type __GroupoidLayoutValue: ::groupoid::TypeLayout;
    type Marker: MetaGroupMarker<Value = Self::Value>;
}
trait MetaGroupMarker {
    type Value;
}
trait MetaGroupMember<
    __GroupoidGroup: MetaGroupMarker,
>: Meta<Marker = __GroupoidGroup, Value = __GroupoidGroup::Value> {}
impl<
    __GroupoidGroup: MetaGroupMarker,
    T: Meta<Marker = __GroupoidGroup, Value = __GroupoidGroup::Value>,
> MetaGroupMember<__GroupoidGroup> for T {}
struct Small;
impl ::groupoid::State for Small {}
struct Big;
impl ::groupoid::State for Big {}
pub struct SmallGroup;
impl ::groupoid::Group for SmallGroup {}
const _: () = if !(::core::mem::size_of::<u32>() == 4) {
    {
        ::core::panicking::panic_fmt(
            format_args!("change `#[size(4)]` on `Value` to the size of `u32`"),
        );
    }
};
impl MetaGroupMarker for SmallGroup {
    type Value = u32;
}
impl Meta for Small {
    type Value = u32;
    type Marker = SmallGroup;
    type __GroupoidLayoutValue = ::groupoid::PinnedTypeLayout<
        SmallGroup,
        u32,
        4,
        { ::core::mem::align_of::<u32>() },
    >;
}
pub struct BigGroup;
impl ::groupoid::Group for BigGroup {}
const _: () = if !(::core::mem::size_of::<i32>() == 4) {
    {
        ::core::panicking::panic_fmt(
            format_args!("change `#[size(4)]` on `Value` to the size of `i32`"),
        );
    }
};
impl MetaGroupMarker for BigGroup {
    type Value = i32;
}
impl Meta for Big {
    type Value = i32;
    type Marker = BigGroup;
    type __GroupoidLayoutValue = ::groupoid::PinnedTypeLayout<
        BigGroup,
        i32,
        4,
        { ::core::mem::align_of::<i32>() },
    >;
}
#[repr(C)]
struct Wrap<S: Meta + ::groupoid::State> {
    value: S::Value,
}
impl<S: Meta + ::groupoid::State> ::groupoid::WithState for Wrap<S> {
    type State = S;
}
unsafe impl<
    S: Meta + ::groupoid::State,
    __GroupoidTargetState: Meta + ::groupoid::State,
> ::groupoid::TransmutableState<__GroupoidTargetState> for Wrap<S>
where
    __GroupoidTargetState::__GroupoidLayoutValue: ::groupoid::SameLayout<
        S::__GroupoidLayoutValue,
    >,
{
    type Target = Wrap<__GroupoidTargetState>;
}
impl<S: Meta + ::groupoid::State> Wrap<S> {
    /// Rebuilds `self` for the target state, converting every
    /// projection through the state with `f`.
    ///
    /// Tuples are rebuilt in place, and every other wrapper goes
    /// through `groupoid::Morph`, one layer at a time. Fields
    /// that don't mention the state move unchanged.
    pub fn morph_with<__GroupoidTargetState: Meta + ::groupoid::State>(
        self,
        mut f: impl FnMut(S::Value) -> __GroupoidTargetState::Value,
    ) -> Wrap<__GroupoidTargetState> {
        Wrap { value: f(self.value) }
    }
}
extern crate test;
#[rustc_test_marker = "transmute_state_between_same_sized_states_of_the_same_struct"]
#[doc(hidden)]
pub const transmute_state_between_same_sized_states_of_the_same_struct: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName(
            "transmute_state_between_same_sized_states_of_the_same_struct",
        ),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "groupoid_macros/tests/transmute_state.rs",
        start_line: 34usize,
        start_col: 4usize,
        end_line: 34usize,
        end_col: 64usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(
            transmute_state_between_same_sized_states_of_the_same_struct(),
        ),
    ),
};
fn transmute_state_between_same_sized_states_of_the_same_struct() {
    let small = Wrap::<Small> {
        value: 0xdead_beefu32,
    };
    let big = unsafe { small.transmute_state::<Big>() };
    let big: Wrap<Big> = big;
    {
        match (&big.value, &(0xdead_beefu32 as i32)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        }
    };
}
extern crate test;
#[rustc_test_marker = "transmute_state_ref_and_mut_round_trip"]
#[doc(hidden)]
pub const transmute_state_ref_and_mut_round_trip: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("transmute_state_ref_and_mut_round_trip"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "groupoid_macros/tests/transmute_state.rs",
        start_line: 44usize,
        start_col: 4usize,
        end_line: 44usize,
        end_col: 42usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(transmute_state_ref_and_mut_round_trip()),
    ),
};
fn transmute_state_ref_and_mut_round_trip() {
    let mut small = Wrap::<Small> { value: 7 };
    {
        let big_ref: &Wrap<Big> = unsafe { small.transmute_state_ref::<Big>() };
        {
            match (&big_ref.value, &7) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            }
        };
    }
    {
        let big_mut: &mut Wrap<Big> = unsafe { small.transmute_state_mut::<Big>() };
        big_mut.value = 9;
    }
    {
        match (&small.value, &9) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        }
    };
}
extern crate test;
#[rustc_test_marker = "target_state_inferred_from_expected_type"]
#[doc(hidden)]
pub const target_state_inferred_from_expected_type: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("target_state_inferred_from_expected_type"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "groupoid_macros/tests/transmute_state.rs",
        start_line: 63usize,
        start_col: 4usize,
        end_line: 63usize,
        end_col: 44usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(target_state_inferred_from_expected_type()),
    ),
};
fn target_state_inferred_from_expected_type() {
    let small = Wrap::<Small> { value: 3 };
    let big: Wrap<Big> = unsafe { small.transmute_state() };
    {
        match (&big.value, &3) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        }
    };
}
#[allow(unused_parens)]
#[repr(C)]
struct Paren<S: Meta + ::groupoid::State> {
    value: (S::Value),
}
impl<S: Meta + ::groupoid::State> ::groupoid::WithState for Paren<S> {
    type State = S;
}
unsafe impl<
    S: Meta + ::groupoid::State,
    __GroupoidTargetState: Meta + ::groupoid::State,
> ::groupoid::TransmutableState<__GroupoidTargetState> for Paren<S>
where
    __GroupoidTargetState::__GroupoidLayoutValue: ::groupoid::SameLayout<
        S::__GroupoidLayoutValue,
    >,
{
    type Target = Paren<__GroupoidTargetState>;
}
impl<S: Meta + ::groupoid::State> Paren<S> {
    /// Rebuilds `self` for the target state, converting every
    /// projection through the state with `f`.
    ///
    /// Tuples are rebuilt in place, and every other wrapper goes
    /// through `groupoid::Morph`, one layer at a time. Fields
    /// that don't mention the state move unchanged.
    pub fn morph_with<__GroupoidTargetState: Meta + ::groupoid::State>(
        self,
        mut f: impl FnMut(S::Value) -> __GroupoidTargetState::Value,
    ) -> Paren<__GroupoidTargetState> {
        Paren { value: f(self.value) }
    }
}
#[repr(C)]
struct ViaMacro<S: Meta + ::groupoid::State> {
    value: S::Value,
}
impl<S: Meta + ::groupoid::State> ::groupoid::WithState for ViaMacro<S> {
    type State = S;
}
unsafe impl<
    S: Meta + ::groupoid::State,
    __GroupoidTargetState: Meta + ::groupoid::State,
> ::groupoid::TransmutableState<__GroupoidTargetState> for ViaMacro<S>
where
    __GroupoidTargetState::__GroupoidLayoutValue: ::groupoid::SameLayout<
        S::__GroupoidLayoutValue,
    >,
{
    type Target = ViaMacro<__GroupoidTargetState>;
}
impl<S: Meta + ::groupoid::State> ViaMacro<S> {
    /// Rebuilds `self` for the target state, converting every
    /// projection through the state with `f`.
    ///
    /// Tuples are rebuilt in place, and every other wrapper goes
    /// through `groupoid::Morph`, one layer at a time. Fields
    /// that don't mention the state move unchanged.
    pub fn morph_with<__GroupoidTargetState: Meta + ::groupoid::State>(
        self,
        mut f: impl FnMut(S::Value) -> __GroupoidTargetState::Value,
    ) -> ViaMacro<__GroupoidTargetState> {
        ViaMacro { value: f(self.value) }
    }
}
extern crate test;
#[rustc_test_marker = "transmute_state_sees_through_parens_and_macro_groups"]
#[doc(hidden)]
pub const transmute_state_sees_through_parens_and_macro_groups: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName(
            "transmute_state_sees_through_parens_and_macro_groups",
        ),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "groupoid_macros/tests/transmute_state.rs",
        start_line: 89usize,
        start_col: 4usize,
        end_line: 89usize,
        end_col: 56usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(
            transmute_state_sees_through_parens_and_macro_groups(),
        ),
    ),
};
fn transmute_state_sees_through_parens_and_macro_groups() {
    let big: Paren<Big> = unsafe {
        Paren::<Small> { value: 3 }.transmute_state::<Big>()
    };
    {
        match (&big.value, &3) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        }
    };
    let big: ViaMacro<Big> = unsafe {
        ViaMacro::<Small> { value: 5 }.transmute_state::<Big>()
    };
    {
        match (&big.value, &5) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        }
    };
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &target_state_inferred_from_expected_type,
            &transmute_state_between_same_sized_states_of_the_same_struct,
            &transmute_state_ref_and_mut_round_trip,
            &transmute_state_sees_through_parens_and_macro_groups,
        ],
    )
}
