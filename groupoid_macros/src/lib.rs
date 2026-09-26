use proc_macro::TokenStream;
use syn::{Ident, ItemImpl, ItemStruct, ItemTrait, parse_macro_input};

use crate::{
    blueprint::Blueprint,
    group::Group,
    group_impl::GroupImpl,
    group_trait::{GroupTrait, GroupTraitArgs},
    state::State,
    typestate::{TypeState, TypeStateArgs},
};

mod blueprint;
mod group;
mod group_impl;
mod group_trait;
mod naming;
mod state;
mod syn_ext;
mod typestate;

/// Declares a trait whose one associated type groups states.
///
/// Adds a `State` supertrait and a `Marker` associated type that
/// `#[group]` fills in.
///
/// ```
/// use groupoid::{blueprint, group, state};
///
/// #[blueprint]
/// trait Meta {
///     type Value;
/// }
///
/// #[state]
/// struct Small;
///
/// #[group(Numbers)]
/// impl Meta for (Small,) {
///     type Value = u32;
/// }
/// # fn main() {}
/// ```
#[proc_macro_attribute]
pub fn blueprint(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_trait = parse_macro_input!(item as ItemTrait);

    Blueprint::new(&item_trait)
        .create_group_marker()
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Implements a `#[blueprint]` trait for every state in the tuple, and
/// declares the named group they belong to.
///
/// `#[size(N)]` on the associated type asserts its size and lets
/// `#[typestate(unsafe_transmute = true)]` transmute between the
/// group's states.
///
/// ```
/// use groupoid::{blueprint, group, state};
///
/// #[blueprint]
/// trait Meta {
///     type Value;
/// }
///
/// #[state]
/// struct Small;
/// #[state]
/// struct Tiny;
///
/// #[group(Words)]
/// impl Meta for (Small, Tiny) {
///     #[size(4)]
///     type Value = u32;
/// }
/// # fn main() {}
/// ```
#[proc_macro_attribute]
pub fn group(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = parse_macro_input!(item as ItemImpl);
    let name = parse_macro_input!(attr as Ident);

    Group::new(&item_impl, &name)
        .generate_group_impl()
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Implements a `#[group_trait]` trait for the types whose state is in
/// the named group.
///
/// See [`macro@group_trait`] for an example.
#[proc_macro_attribute]
pub fn group_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = parse_macro_input!(item as ItemImpl);
    let name = parse_macro_input!(attr as Ident);

    GroupImpl::new(&item_impl, &name)
        .create_group_impl()
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Marks a generic struct as a typestate container.
///
/// Implements `WithState`, and `restate_with` when a field projects
/// through the state. `unsafe_transmute = true` also implements
/// `TransmutableState`, and `align = N` forces the alignment it uses.
///
/// ```
/// use groupoid::{blueprint, group, state, typestate};
///
/// #[blueprint]
/// trait Meta {
///     type Value;
/// }
///
/// #[state]
/// struct Small;
/// #[state]
/// struct Big;
///
/// #[group(SmallGroup)]
/// impl Meta for (Small,) {
///     type Value = u8;
/// }
///
/// #[group(BigGroup)]
/// impl Meta for (Big,) {
///     type Value = u64;
/// }
///
/// #[typestate]
/// struct Wrap<S: Meta> {
///     value: S::Value,
/// }
///
/// fn main() {
///     let small = Wrap::<Small> { value: 7 };
///     let big: Wrap<Big> = small.restate_with(u64::from);
///     assert_eq!(big.value, 7);
/// }
/// ```
#[proc_macro_attribute]
pub fn typestate(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_struct = parse_macro_input!(item as ItemStruct);
    let args = parse_macro_input!(attr as TypeStateArgs);

    TypeState::new(args, item_struct)
        .map(|typestate| typestate.generate_typestate_impls())
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Implements `groupoid::State` for a struct.
///
/// ```
/// use groupoid::state;
///
/// #[state]
/// struct Small;
/// # fn main() {}
/// ```
#[proc_macro_attribute]
pub fn state(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_struct = parse_macro_input!(item as ItemStruct);

    State::new(&item_struct).generate_state_impl().into()
}

/// Declares a trait whose implementation depends on the group of the
/// implementer's state. Each group implements it with `#[group_impl]`.
///
/// ```
/// use groupoid::{
///     blueprint, group, group_impl, group_trait, state, typestate,
/// };
///
/// #[blueprint]
/// trait Meta {
///     type Value;
/// }
///
/// #[state]
/// struct Small;
/// #[state]
/// struct Big;
///
/// #[group(Numbers)]
/// impl Meta for (Small,) {
///     type Value = u32;
/// }
///
/// #[group(Words)]
/// impl Meta for (Big,) {
///     type Value = String;
/// }
///
/// #[typestate]
/// struct Wrap<S: Meta> {
///     value: S::Value,
/// }
///
/// #[group_trait(by = Meta)]
/// trait Describe {
///     fn describe(&self) -> String;
/// }
///
/// #[group_impl(Numbers)]
/// impl<S: Meta<Value = u32>> Describe for Wrap<S> {
///     fn describe(&self) -> String {
///         format!("number {}", self.value)
///     }
/// }
///
/// #[group_impl(Words)]
/// impl<S: Meta<Value = String>> Describe for Wrap<S> {
///     fn describe(&self) -> String {
///         format!("word {}", self.value)
///     }
/// }
///
/// fn main() {
///     assert_eq!(Wrap::<Small> { value: 1 }.describe(), "number 1");
///     let big = Wrap::<Big> { value: "hi".into() };
///     assert_eq!(big.describe(), "word hi");
/// }
/// ```
#[proc_macro_attribute]
pub fn group_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_trait = parse_macro_input!(item as ItemTrait);
    let args = parse_macro_input!(attr as GroupTraitArgs);

    GroupTrait::new(&args, &item_trait)
        .generate_group_trait()
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}
