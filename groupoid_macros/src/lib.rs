use proc_macro::TokenStream;
use syn::{Ident, ItemImpl, ItemStruct, ItemTrait, parse_macro_input};

use crate::{
    group::Group,
    group_impl::{GroupImpl, GroupImplArgs},
    group_trait::{GroupTrait, GroupTraitArgs},
    state::State,
    template::Template,
    typestate::{TypeState, TypeStateArgs},
};

mod group;
mod group_impl;
mod group_trait;
mod naming;
mod state;
mod syn_ext;
mod template;
mod typestate;

/// Declares a trait whose associated types group states.
///
/// The trait needs one or more associated types. `#[template]` adds a
/// `State` supertrait and a `Marker` associated type that `#[group]`
/// fills in. It also generates `{Trait}Morph<Src, Dst>`, with one method
/// per associated type. Implement it to convert between two states, then
/// pass it to `morph` on a `#[typestate]` struct.
///
/// ```
/// use groupoid::{group, state, template};
///
/// #[template]
/// trait Meta {
///     type Value;
///     type Label;
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
///     type Label = &'static str;
/// }
///
/// #[group(Wide)]
/// impl Meta for (Big,) {
///     type Value = u64;
///     type Label = String;
/// }
///
/// struct Widen;
///
/// impl MetaMorph<Small, Big> for Widen {
///     fn value(&mut self, value: u32) -> u64 {
///         value.into()
///     }
///
///     fn label(&mut self, label: &'static str) -> String {
///         label.to_uppercase()
///     }
/// }
/// # fn main() {}
/// ```
#[proc_macro_attribute]
pub fn template(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_trait = parse_macro_input!(item as ItemTrait);

    Template::new(&item_trait)
        .and_then(|template| template.create_group_marker())
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Implements a `#[template]` trait for every state in the tuple, and
/// declares the named group they belong to.
///
/// `#[size(N)]` on an associated type asserts its size. With it,
/// `#[typestate(unsafe_transmute = true)]` can transmute a struct that
/// uses that type into another state whose type has the same size.
///
/// ```
/// use groupoid::{group, state, template};
///
/// #[template]
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
/// The impl's only generic type parameter is the state; with several,
/// name it with `#[group_impl(Group, state = S)]`. The group sets the
/// state's associated type, so bound the state by the template alone.
///
/// See [`macro@group_trait`] for an example.
#[proc_macro_attribute]
pub fn group_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = parse_macro_input!(item as ItemImpl);
    let args = parse_macro_input!(attr as GroupImplArgs);

    GroupImpl::new(&args, &item_impl)
        .and_then(|group_impl| group_impl.create_group_impl())
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Marks a generic struct as a typestate container.
///
/// Implements `WithState`. When a field projects through the state, it
/// also generates two methods that rebuild the struct in another state:
///
/// - `morph_with` takes one closure for a single projection, and a
///   generated `{Struct}Morph { .. }` with one closure per projection
///   otherwise.
/// - `morph` takes any value implementing the template's
///   `{Template}Morph`.
///
/// `unsafe_transmute = true` also implements `TransmutableState` for
/// every state whose projections have the same `#[size(N)]`, and
/// `align = N` forces the alignment it uses.
///
/// ```
/// use groupoid::{group, state, template, typestate};
///
/// #[template]
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
///     let big: Wrap<Big> = small.morph_with(u64::from);
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
///     group, group_impl, group_trait, state, template, typestate,
/// };
///
/// #[template]
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
/// impl<S: Meta> Describe for Wrap<S> {
///     fn describe(&self) -> String {
///         format!("number {}", self.value)
///     }
/// }
///
/// #[group_impl(Words)]
/// impl<S: Meta> Describe for Wrap<S> {
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
