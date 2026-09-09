use proc_macro::TokenStream;
use syn::{Ident, ItemImpl, ItemStruct, ItemTrait, parse_macro_input};

use crate::{
    blueprint::Blueprint,
    group::Group,
    group_impl::GroupImpl,
    group_trait::{GroupTrait, GroupTraitArgs},
    state::{State, StateArg},
};

mod blueprint;
mod group;
mod group_impl;
mod group_trait;
mod proto;
mod state;
// pub mod syntax_prototype;

#[proc_macro_attribute]
pub fn blueprint(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_trait = parse_macro_input!(item as ItemTrait);

    Blueprint::new(&item_trait).create_group_marker().into()
}

#[proc_macro_attribute]
pub fn group(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = parse_macro_input!(item as ItemImpl);
    let name = parse_macro_input!(attr as Ident);

    Group::new(&item_impl, &name)
        .generate_group_impl()
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn group_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = parse_macro_input!(item as ItemImpl);
    let name = parse_macro_input!(attr as Ident);

    GroupImpl::new(&item_impl, &name)
        .create_group_impl()
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn state(attr: TokenStream, item: TokenStream) -> TokenStream {
    let state = parse_macro_input!(attr as State);
    let item_struct = parse_macro_input!(item as ItemStruct);

    StateArg::new(&state, &item_struct)
        .generate_has_state_impl()
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn group_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as GroupTraitArgs);
    let item_trait = parse_macro_input!(item as ItemTrait);

    GroupTrait::new(&args, &item_trait)
        .generate_group_trait()
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}
