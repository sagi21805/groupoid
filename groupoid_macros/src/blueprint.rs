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

    pub fn create_group_marker(&self) -> syn::Result<TokenStream> {
        let trait_ident = &self.inner.ident;
        let marker_name = format_ident!("{}GroupMarker", trait_ident);
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

        // Exactly one associated type keeps `#[group]`'s `#[size(N)]` (and
        // the `::groupoid::SizedGroup<SIZE>` it implements) unambiguous
        // about which type it's describing, with no discriminator needed.
        let [type_definition] = *type_definitions.as_slice() else {
            return Err(syn::Error::new_spanned(
                &self.inner.ident,
                format!(
                    "`#[blueprint]` requires exactly one associated \
                     type, found {}",
                    type_definitions.len()
                ),
            ));
        };
        let assoc_ident = &type_definition.ident;

        let mut original = self.inner.clone();

        let marker_type = parse_quote! {
            type Marker: #marker_name<#assoc_ident = Self::#assoc_ident>;
        };

        original.items.push(TraitItem::Type(marker_type));

        Ok(quote! {

            #original

            trait #marker_name {
                #type_definition
            }
        })
    }
}
