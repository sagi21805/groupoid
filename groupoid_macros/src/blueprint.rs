use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemTrait, TraitItem, TraitItemType, parse_quote};

pub struct Blueprint<'ast> {
    inner: &'ast ItemTrait,
}

impl<'ast> Blueprint<'ast> {
    pub fn new(inner: &'ast ItemTrait) -> Blueprint<'ast> {
        Blueprint { inner }
    }

    pub fn create_group_marker(&self) -> TokenStream {
        let marker_name = format_ident!("{}GroupMarker", &self.inner.ident);
        let type_definitions: Vec<&TraitItemType> = self
            .inner
            .items
            .iter()
            .filter_map(|i| {
                if let TraitItem::Type(t) = i {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();

        let type_idents = type_definitions.iter().map(|t| &t.ident);

        let mut original = self.inner.clone();

        let marker_type = parse_quote! {
            type Marker: #marker_name<
                #( #type_idents = Self::#type_idents ),*
            >;
        };

        original.items.push(TraitItem::Type(marker_type));

        quote! {

            #original

            trait #marker_name {
                #(#type_definitions)*
            }
        }
    }
}
