use proc_macro::TokenStream;
use syn::{Ident, ItemImpl, ItemTrait, parse_macro_input};

use crate::{blueprint::Blueprint, group_impl::GroupImpl};

mod blueprint;
mod group_impl;
mod proto;
// pub mod syntax_prototype;

#[proc_macro_attribute]
pub fn blueprint(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_trait = parse_macro_input!(item as ItemTrait);

    Blueprint::new(&item_trait).create_group_marker().into()
}

#[proc_macro_attribute]
pub fn group_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_trait = parse_macro_input!(item as ItemImpl);
    let name = parse_macro_input!(attr as Ident);

    GroupImpl::new(&item_trait, &name)
        .generate_group_impl()
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
