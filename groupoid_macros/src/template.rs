use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemTrait, TraitItem, TraitItemType, parse_quote};

pub struct Template<'ast> {
    inner: &'ast ItemTrait,
}

impl<'ast> Template<'ast> {
    pub fn new(inner: &'ast ItemTrait) -> Template<'ast> {
        Template { inner }
    }

    /// The trait with a `Marker` associated type and a `State`
    /// supertrait, followed by its `{Trait}GroupMarker` trait and its
    /// `{Trait}GroupMember<G>` trait, which every state of group `G`
    /// implements.
    pub fn create_group_marker(&self) -> syn::Result<TokenStream> {
        let vis = &self.inner.vis;
        let trait_ident = &self.inner.ident;
        let marker_name = crate::naming::group_marker_ident(trait_ident);
        let member_name = crate::naming::group_member_ident(trait_ident);
        let group = crate::naming::group_param_ident();
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
                    "declare exactly one associated type in a \
                     `#[template]` trait; this one has {}",
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

        let member_bound = quote! {
            #trait_ident<Marker = #group, #assoc_ident = #group::#assoc_ident>
        };

        Ok(quote! {
            #original

            #vis trait #marker_name {
                #type_definition
            }

            #vis trait #member_name<#group: #marker_name>: #member_bound {}

            impl<#group: #marker_name, T: #member_bound> #member_name<#group> for T {}
        })
    }
}
