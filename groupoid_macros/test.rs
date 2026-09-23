#![feature(prelude_import)]
//! The by-value transition `#[typestate]` generates next to the in-place
//! transmute: `restate_with`, where the caller converts each projection. It
//! rebuilds the struct field by field, so it exists for every struct that
//! projects through its state, including the shapes that disqualify the
//! in-place path (`Option<S::Value>` and friends).
//!
//! The macro sees through `Option`, arrays, `Box` and tuples by itself; any
//! other wrapper goes through a user `Restate` impl.
extern crate std;
use core::marker::PhantomData;
use groupoid::Restate;
use groupoid_macros::{blueprint, group, state, typestate};
#[prelude_import]
use std::prelude::rust_2024::*;
trait Meta {
    type Value;
    type Marker: MetaGroupMarker<Value = Self::Value>;
}
trait MetaGroupMarker {
    type Value;
}
struct Small;
impl ::groupoid::State for Small {}
struct Big;
impl ::groupoid::State for Big {}
struct SmallGroup;
impl ::groupoid::Group for SmallGroup {}
const _: () = if !(::core::mem::size_of::<u32>() == 4) {
    {
        ::core::panicking::panic_fmt(format_args!(
            "associated type `Value` is `u32`, which is not 4 byte(s)"
        ));
    }
};
impl ::groupoid::SizedGroup<4> for SmallGroup {}
impl ::groupoid::AlignedGroup<{ ::core::mem::align_of::<u32>() }> for SmallGroup {}
impl MetaGroupMarker for SmallGroup {
    type Value = u32;
}
impl Meta for Small {
    type Value = u32;
    type Marker = SmallGroup;
}
struct BigGroup;
impl ::groupoid::Group for BigGroup {}
const _: () = if !(::core::mem::size_of::<i32>() == 4) {
    {
        ::core::panicking::panic_fmt(format_args!(
            "associated type `Value` is `i32`, which is not 4 byte(s)"
        ));
    }
};
impl ::groupoid::SizedGroup<4> for BigGroup {}
impl ::groupoid::AlignedGroup<{ ::core::mem::align_of::<i32>() }> for BigGroup {}
impl MetaGroupMarker for BigGroup {
    type Value = i32;
}
impl Meta for Big {
    type Value = i32;
    type Marker = BigGroup;
}
struct Pair<T>(T, T);
#[automatically_derived]
impl<T: ::core::fmt::Debug> ::core::fmt::Debug for Pair<T> {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_tuple_field2_finish(f, "Pair", &self.0, &&self.1)
    }
}
#[automatically_derived]
impl<T: ::core::cmp::PartialEq> ::core::marker::StructuralPartialEq for Pair<T> {}
#[automatically_derived]
impl<T: ::core::cmp::PartialEq> ::core::cmp::PartialEq for Pair<T> {
    #[inline]
    fn eq(&self, other: &Pair<T>) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}
impl<A, B> Restate<A, B> for Pair<A> {
    type Output = Pair<B>;
    fn restate(self, leaf: &mut impl FnMut(A) -> B) -> Pair<B> {
        Pair(leaf(self.0), leaf(self.1))
    }
}
struct Wrap<S: Meta + ::groupoid::State> {
    value: S::Value,
    maybe: Option<S::Value>,
    many: [S::Value; 2],
    boxed: Box<S::Value>,
    pair: (S::Value, u8),
    nested: Option<[S::Value; 2]>,
    user: Pair<S::Value>,
    tag: u8,
    _s: PhantomData<S>,
}
impl<S: Meta + ::groupoid::State> ::groupoid::WithState for Wrap<S> {
    type State = S;
}
impl<S: Meta + ::groupoid::State> Wrap<S> {
    /// Rebuilds `self` for the target state by value, converting
    /// every projection through the state with `leaf`.
    ///
    /// A projection is reached through any nesting of `Option`,
    /// arrays, `Box` and tuples; any other wrapper around one is
    /// handed to `groupoid::Restate`. Fields that do not
    /// mention the state are moved as they are.
    pub fn restate_with<__GroupoidTargetState: Meta + ::groupoid::State>(
        self,
        mut __groupoid_leaf: impl FnMut(S::Value) -> __GroupoidTargetState::Value,
    ) -> Wrap<__GroupoidTargetState>
    where
        Pair<S::Value>: ::groupoid::Restate<
                S::Value,
                __GroupoidTargetState::Value,
                Output = Pair<__GroupoidTargetState::Value>,
            >,
    {
        Wrap {
            value: __groupoid_leaf(self.value),
            maybe: self.maybe.map(|__groupoid_elem| __groupoid_leaf(__groupoid_elem)),
            many: self.many.map(|__groupoid_elem| __groupoid_leaf(__groupoid_elem)),
            boxed: ::std::boxed::Box::new({
                let __groupoid_elem = *self.boxed;
                __groupoid_leaf(__groupoid_elem)
            }),
            pair: {
                let (__groupoid_elem0, __groupoid_elem1) = self.pair;
                (__groupoid_leaf(__groupoid_elem0), __groupoid_elem1)
            },
            nested: self
                .nested
                .map(|__groupoid_elem| {
                    __groupoid_elem
                        .map(|__groupoid_elem| __groupoid_leaf(__groupoid_elem))
                }),
            user: <Pair<
                S::Value,
            > as ::groupoid::Restate<
                S::Value,
                __GroupoidTargetState::Value,
            >>::restate(self.user, &mut __groupoid_leaf),
            tag: self.tag,
            _s: ::core::marker::PhantomData,
        }
    }
}
fn sample() -> Wrap<Small> {
    Wrap {
        value: 1,
        maybe: Some(2),
        many: [3, 4],
        boxed: Box::new(5),
        pair: (6, 7),
        nested: Some([8, 9]),
        user: Pair(10, 11),
        tag: 12,
        _s: PhantomData,
    }
}
fn assert_round_trip(big: Wrap<Big>) {
    {
        match (&big.value, &1) {
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
    {
        match (&big.maybe, &Some(2)) {
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
    {
        match (&big.many, &[3, 4]) {
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
    {
        match (&*big.boxed, &5) {
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
    {
        match (&big.pair, &(6, 7)) {
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
    {
        match (&big.nested, &Some([8, 9])) {
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
    {
        match (&big.user, &Pair(10, 11)) {
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
    {
        match (&big.tag, &12) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::Some(format_args!(
                            "the state-independent field is moved as-is"
                        )),
                    );
                }
            }
        }
    };
}
extern crate test;
#[rustc_test_marker = "restate_with_converts_every_projection_through_the_leaf"]
#[doc(hidden)]
pub const restate_with_converts_every_projection_through_the_leaf: test::TestDescAndFn =
    test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("restate_with_converts_every_projection_through_the_leaf"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "groupoid_macros/tests/restate.rs",
            start_line: 90usize,
            start_col: 4usize,
            end_line: 90usize,
            end_col: 59usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::IntegrationTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(restate_with_converts_every_projection_through_the_leaf()),
        ),
    };
#[rustc_test_entrypoint_marker]
fn restate_with_converts_every_projection_through_the_leaf() {
    let mut calls = 0;
    let big: Wrap<Big> = sample().restate_with(|v| {
        calls += 1;
        v as i32
    });
    assert_round_trip(big);
    {
        match (&calls, &10) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::Some(format_args!(
                            "one call per projection: value, maybe, many x2, boxed, pair.0, \
                             nested x2, user x2",
                        )),
                    );
                }
            }
        }
    };
}
extern crate test;
#[rustc_test_marker = "restate_with_sees_none_without_calling_the_leaf"]
#[doc(hidden)]
pub const restate_with_sees_none_without_calling_the_leaf: test::TestDescAndFn =
    test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("restate_with_sees_none_without_calling_the_leaf"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "groupoid_macros/tests/restate.rs",
            start_line: 104usize,
            start_col: 4usize,
            end_line: 104usize,
            end_col: 51usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::IntegrationTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(restate_with_sees_none_without_calling_the_leaf()),
        ),
    };
#[rustc_test_entrypoint_marker]
fn restate_with_sees_none_without_calling_the_leaf() {
    let small = Wrap::<Small> {
        maybe: None,
        nested: None,
        ..sample()
    };
    let big: Wrap<Big> = small.restate_with(|v| v as i32);
    {
        match (&big.maybe, &None) {
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
    {
        match (&big.nested, &None) {
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
struct NotTransmutable<S: Meta + ::groupoid::State> {
    value: Option<S::Value>,
}
impl<S: Meta + ::groupoid::State> ::groupoid::WithState for NotTransmutable<S> {
    type State = S;
}
impl<S: Meta + ::groupoid::State> NotTransmutable<S> {
    /// Rebuilds `self` for the target state by value, converting
    /// every projection through the state with `leaf`.
    ///
    /// A projection is reached through any nesting of `Option`,
    /// arrays, `Box` and tuples; any other wrapper around one is
    /// handed to `groupoid::Restate`. Fields that do not
    /// mention the state are moved as they are.
    pub fn restate_with<__GroupoidTargetState: Meta + ::groupoid::State>(
        self,
        mut __groupoid_leaf: impl FnMut(S::Value) -> __GroupoidTargetState::Value,
    ) -> NotTransmutable<__GroupoidTargetState> {
        NotTransmutable {
            value: self
                .value
                .map(|__groupoid_elem| __groupoid_leaf(__groupoid_elem)),
        }
    }
}
extern crate test;
#[rustc_test_marker = "a_struct_without_the_in_place_path_still_restates_by_value"]
#[doc(hidden)]
pub const a_struct_without_the_in_place_path_still_restates_by_value: test::TestDescAndFn =
    test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName(
                "a_struct_without_the_in_place_path_still_restates_by_value",
            ),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "groupoid_macros/tests/restate.rs",
            start_line: 124usize,
            start_col: 4usize,
            end_line: 124usize,
            end_col: 62usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::IntegrationTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || {
                test::assert_test_result(
                    a_struct_without_the_in_place_path_still_restates_by_value(),
                )
            },
        ),
    };
#[rustc_test_entrypoint_marker]
fn a_struct_without_the_in_place_path_still_restates_by_value() {
    let big: NotTransmutable<Big> = NotTransmutable::<Small> {
        value: Some(0xdead_beef),
    }
    .restate_with(|v| v as i32);
    {
        match (&big.value, &Some(0xdead_beefu32 as i32)) {
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
struct Ordered<S: Meta + ::groupoid::State, T> {
    head: u16,
    value: S::Value,
    tail: T,
}
impl<S: Meta + ::groupoid::State, T> ::groupoid::WithState for Ordered<S, T> {
    type State = S;
}
impl<S: Meta + ::groupoid::State, T> Ordered<S, T> {
    /// Rebuilds `self` for the target state by value, converting
    /// every projection through the state with `leaf`.
    ///
    /// A projection is reached through any nesting of `Option`,
    /// arrays, `Box` and tuples; any other wrapper around one is
    /// handed to `groupoid::Restate`. Fields that do not
    /// mention the state are moved as they are.
    pub fn restate_with<__GroupoidTargetState: Meta + ::groupoid::State>(
        self,
        mut __groupoid_leaf: impl FnMut(S::Value) -> __GroupoidTargetState::Value,
    ) -> Ordered<__GroupoidTargetState, T> {
        Ordered {
            head: self.head,
            value: __groupoid_leaf(self.value),
            tail: self.tail,
        }
    }
}
extern crate test;
#[rustc_test_marker = "other_generics_carry_over_to_the_target"]
#[doc(hidden)]
pub const other_generics_carry_over_to_the_target: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("other_generics_carry_over_to_the_target"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "groupoid_macros/tests/restate.rs",
        start_line: 142usize,
        start_col: 4usize,
        end_line: 142usize,
        end_col: 43usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(other_generics_carry_over_to_the_target()),
    ),
};
#[rustc_test_entrypoint_marker]
fn other_generics_carry_over_to_the_target() {
    let small = Ordered::<Small, u64> {
        head: 1,
        value: 2,
        tail: 3,
    };
    let big: Ordered<Big, u64> = small.restate_with(|v| v as i32);
    {
        match (&(big.head, big.value, big.tail), &(1, 2, 3)) {
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
struct Positional<S: Meta + ::groupoid::State>(u8, S::Value, Option<S::Value>);
impl<S: Meta + ::groupoid::State> ::groupoid::WithState for Positional<S> {
    type State = S;
}
impl<S: Meta + ::groupoid::State> Positional<S> {
    /// Rebuilds `self` for the target state by value, converting
    /// every projection through the state with `leaf`.
    ///
    /// A projection is reached through any nesting of `Option`,
    /// arrays, `Box` and tuples; any other wrapper around one is
    /// handed to `groupoid::Restate`. Fields that do not
    /// mention the state are moved as they are.
    pub fn restate_with<__GroupoidTargetState: Meta + ::groupoid::State>(
        self,
        mut __groupoid_leaf: impl FnMut(S::Value) -> __GroupoidTargetState::Value,
    ) -> Positional<__GroupoidTargetState> {
        Positional(
            self.0,
            __groupoid_leaf(self.1),
            self.2
                .map(|__groupoid_elem| __groupoid_leaf(__groupoid_elem)),
        )
    }
}
extern crate test;
#[rustc_test_marker = "tuple_structs_are_rebuilt_positionally"]
#[doc(hidden)]
pub const tuple_structs_are_rebuilt_positionally: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("tuple_structs_are_rebuilt_positionally"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "groupoid_macros/tests/restate.rs",
        start_line: 156usize,
        start_col: 4usize,
        end_line: 156usize,
        end_col: 42usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(tuple_structs_are_rebuilt_positionally()),
    ),
};
#[rustc_test_entrypoint_marker]
fn tuple_structs_are_rebuilt_positionally() {
    let big: Positional<Big> = Positional::<Small>(1, 2, Some(3)).restate_with(|v| v as i32);
    {
        match (&(big.0, big.1, big.2), &(1, 2, Some(3))) {
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
struct Wide;
impl ::groupoid::State for Wide {}
struct Bytes;
impl ::groupoid::State for Bytes {}
struct WideGroup;
impl ::groupoid::Group for WideGroup {}
const _: () = if !(::core::mem::size_of::<u64>() == 8) {
    {
        ::core::panicking::panic_fmt(format_args!(
            "associated type `Value` is `u64`, which is not 8 byte(s)"
        ));
    }
};
impl ::groupoid::SizedGroup<8> for WideGroup {}
impl ::groupoid::AlignedGroup<{ ::core::mem::align_of::<u64>() }> for WideGroup {}
impl MetaGroupMarker for WideGroup {
    type Value = u64;
}
impl Meta for Wide {
    type Value = u64;
    type Marker = WideGroup;
}
struct BytesGroup;
impl ::groupoid::Group for BytesGroup {}
const _: () = if !(::core::mem::size_of::<[u8; 8]>() == 8) {
    {
        ::core::panicking::panic_fmt(format_args!(
            "associated type `Value` is `[u8; 8]`, which is not 8 byte(s)"
        ));
    }
};
impl ::groupoid::SizedGroup<8> for BytesGroup {}
impl ::groupoid::AlignedGroup<{ ::core::mem::align_of::<[u8; 8]>() }> for BytesGroup {}
impl MetaGroupMarker for BytesGroup {
    type Value = [u8; 8];
}
impl Meta for Bytes {
    type Value = [u8; 8];
    type Marker = BytesGroup;
}
#[repr(C)]
#[repr(align(8))]
struct Forced<S: Meta + ::groupoid::State> {
    value: S::Value,
    tag: u8,
}
impl<S: Meta + ::groupoid::State> ::groupoid::WithState for Forced<S> {
    type State = S;
}
impl<S: Meta + ::groupoid::State, const __GROUPOID_SIZE: usize>
    ::groupoid::SizedWithState<__GROUPOID_SIZE, 8> for Forced<S>
where
    S::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>,
{
}
unsafe impl<
    S: Meta + ::groupoid::State,
    __GroupoidTargetState: Meta + ::groupoid::State,
    const __GROUPOID_SIZE: usize,
> ::groupoid::TransmutableState<__GroupoidTargetState, __GROUPOID_SIZE, 8> for Forced<S>
where
    S::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>,
    __GroupoidTargetState::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>,
{
    type Target = Forced<__GroupoidTargetState>;
}
impl<S: Meta + ::groupoid::State> Forced<S> {
    /// Rebuilds `self` for the target state by value, converting
    /// every projection through the state with `leaf`.
    ///
    /// A projection is reached through any nesting of `Option`,
    /// arrays, `Box` and tuples; any other wrapper around one is
    /// handed to `groupoid::Restate`. Fields that do not
    /// mention the state are moved as they are.
    pub fn restate_with<__GroupoidTargetState: Meta + ::groupoid::State>(
        self,
        mut __groupoid_leaf: impl FnMut(S::Value) -> __GroupoidTargetState::Value,
    ) -> Forced<__GroupoidTargetState> {
        Forced {
            value: __groupoid_leaf(self.value),
            tag: self.tag,
        }
    }
}
extern crate test;
#[rustc_test_marker = "forced_alignment_mode_still_generates_restate_with"]
#[doc(hidden)]
pub const forced_alignment_mode_still_generates_restate_with: test::TestDescAndFn =
    test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("forced_alignment_mode_still_generates_restate_with"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "groupoid_macros/tests/restate.rs",
            start_line: 187usize,
            start_col: 4usize,
            end_line: 187usize,
            end_col: 54usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::IntegrationTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(forced_alignment_mode_still_generates_restate_with()),
        ),
    };
#[rustc_test_entrypoint_marker]
fn forced_alignment_mode_still_generates_restate_with() {
    let wide = Forced::<Wide> {
        value: u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
        tag: 9,
    };
    let bytes: Forced<Bytes> = wide.restate_with(u64::to_ne_bytes);
    {
        match (&bytes.value, &[1, 2, 3, 4, 5, 6, 7, 8]) {
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
    {
        match (&bytes.tag, &9) {
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
struct Unforced<S: Meta + ::groupoid::State> {
    value: S::Value,
    tag: u8,
}
impl<S: Meta + ::groupoid::State> ::groupoid::WithState for Unforced<S> {
    type State = S;
}
impl<S: Meta + ::groupoid::State> Unforced<S> {
    /// Rebuilds `self` for the target state by value, converting
    /// every projection through the state with `leaf`.
    ///
    /// A projection is reached through any nesting of `Option`,
    /// arrays, `Box` and tuples; any other wrapper around one is
    /// handed to `groupoid::Restate`. Fields that do not
    /// mention the state are moved as they are.
    pub fn restate_with<__GroupoidTargetState: Meta + ::groupoid::State>(
        self,
        mut __groupoid_leaf: impl FnMut(S::Value) -> __GroupoidTargetState::Value,
    ) -> Unforced<__GroupoidTargetState> {
        Unforced {
            value: __groupoid_leaf(self.value),
            tag: self.tag,
        }
    }
}
extern crate test;
#[rustc_test_marker = "restate_with_ignores_alignment_altogether"]
#[doc(hidden)]
pub const restate_with_ignores_alignment_altogether: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("restate_with_ignores_alignment_altogether"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "groupoid_macros/tests/restate.rs",
        start_line: 208usize,
        start_col: 4usize,
        end_line: 208usize,
        end_col: 45usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(restate_with_ignores_alignment_altogether()),
    ),
};
#[rustc_test_entrypoint_marker]
fn restate_with_ignores_alignment_altogether() {
    let wide = Unforced::<Wide> {
        value: u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
        tag: 9,
    };
    let bytes: Unforced<Bytes> = wide.restate_with(u64::to_ne_bytes);
    {
        match (&bytes.value, &[1, 2, 3, 4, 5, 6, 7, 8]) {
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
    {
        match (&bytes.tag, &9) {
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
#[repr(Rust)]
struct Unpinned<S: Meta + ::groupoid::State> {
    value: S::Value,
    tag: u8,
}
impl<S: Meta + ::groupoid::State> ::groupoid::WithState for Unpinned<S> {
    type State = S;
}
impl<S: Meta + ::groupoid::State> Unpinned<S> {
    /// Rebuilds `self` for the target state by value, converting
    /// every projection through the state with `leaf`.
    ///
    /// A projection is reached through any nesting of `Option`,
    /// arrays, `Box` and tuples; any other wrapper around one is
    /// handed to `groupoid::Restate`. Fields that do not
    /// mention the state are moved as they are.
    pub fn restate_with<__GroupoidTargetState: Meta + ::groupoid::State>(
        self,
        mut __groupoid_leaf: impl FnMut(S::Value) -> __GroupoidTargetState::Value,
    ) -> Unpinned<__GroupoidTargetState> {
        Unpinned {
            value: __groupoid_leaf(self.value),
            tag: self.tag,
        }
    }
}
extern crate test;
#[rustc_test_marker = "restate_only_structs_keep_their_own_repr"]
#[doc(hidden)]
pub const restate_only_structs_keep_their_own_repr: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("restate_only_structs_keep_their_own_repr"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "groupoid_macros/tests/restate.rs",
        start_line: 231usize,
        start_col: 4usize,
        end_line: 231usize,
        end_col: 44usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(restate_only_structs_keep_their_own_repr()),
    ),
};
#[rustc_test_entrypoint_marker]
fn restate_only_structs_keep_their_own_repr() {
    let big: Unpinned<Big> = Unpinned::<Small> { value: 7, tag: 9 }.restate_with(|v| v as i32);
    {
        match (&(big.value, big.tag), &(7, 9)) {
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
    test::test_main_static(&[
        &a_struct_without_the_in_place_path_still_restates_by_value,
        &forced_alignment_mode_still_generates_restate_with,
        &other_generics_carry_over_to_the_target,
        &restate_only_structs_keep_their_own_repr,
        &restate_with_converts_every_projection_through_the_leaf,
        &restate_with_ignores_alignment_altogether,
        &restate_with_sees_none_without_calling_the_leaf,
        &tuple_structs_are_rebuilt_positionally,
    ])
}
