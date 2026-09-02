use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemTrait, TraitItem};

pub struct Blueprint<'ast> {
    inner: &'ast ItemTrait,
}

impl<'ast> Blueprint<'ast> {
    pub fn new(inner: &'ast ItemTrait) -> Blueprint<'ast> {
        Blueprint { inner }
    }

    pub fn create_group_marker(&self) -> TokenStream {
        let marker_name = format_ident!("{}GroupMarker", &self.inner.ident);
        let type_definitions: Vec<&TraitItem> = self
            .inner
            .items
            .iter()
            .filter(|i| matches!(i, TraitItem::Type(_)))
            .collect();

        quote! {
            trait #marker_name {
                #(#type_definitions)*
            }
        }
    }
}
