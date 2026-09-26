use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemTrait, TraitItem, TraitItemType, parse_quote};

pub struct Blueprint<'ast> {
    inner: &'ast ItemTrait,
}

impl<'ast> Blueprint<'ast> {
    pub fn new(inner: &'ast ItemTrait) -> Blueprint<'ast> {
        Blueprint { inner }
    }

    /// The trait with a `Marker` associated type and a `State`
    /// supertrait, followed by its `{Trait}GroupMarker` trait.
    pub fn create_group_marker(&self) -> syn::Result<TokenStream> {
        let vis = &self.inner.vis;
        let marker_name =
            crate::naming::group_marker_ident(&self.inner.ident);
        let type_definitions: Vec<&TraitItemType> = self
            .inner
            .items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Type(ty) => Some(ty),
                _ => None,
            })
            .collect();

        // One associated type keeps `#[size(N)]` in `#[group]`
        // unambiguous.
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
        original.items.push(parse_quote! {
            type Marker: #marker_name<#assoc_ident = Self::#assoc_ident>;
        });
        original.supertraits.push(parse_quote!(::groupoid::State));

        Ok(quote! {
            #original

            #vis trait #marker_name {
                #type_definition
            }
        })
    }
}
