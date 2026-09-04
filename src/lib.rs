use proc_macro::TokenStream;
use syn::{ItemTrait, parse_macro_input};

use crate::blueprint::Blueprint;

mod blueprint;
mod proto;
// pub mod syntax_prototype;

#[proc_macro_attribute]
pub fn blueprint(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_trait = parse_macro_input!(item as ItemTrait);

    Blueprint::new(&item_trait).create_group_marker().into()
}
