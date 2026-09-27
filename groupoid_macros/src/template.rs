use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemTrait, TraitItem, TraitItemType, parse_quote};

pub struct Template<'ast> {
    inner: &'ast ItemTrait,
    /// The trait's associated types, one or more.
    types: Vec<&'ast TraitItemType>,
}

impl<'ast> Template<'ast> {
    pub fn new(inner: &'ast ItemTrait) -> syn::Result<Template<'ast>> {
        let types: Vec<&TraitItemType> = inner
            .items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Type(ty) => Some(ty),
                _ => None,
            })
            .collect();

        if types.is_empty() {
            return Err(syn::Error::new_spanned(
                &inner.ident,
                format!(
                    "add an associated type to `{}`, such as `type \
                     Value;`, for its groups to set",
                    inner.ident
                ),
            ));
        }

        Ok(Template { inner, types })
    }

    /// The trait plus its `{Trait}GroupMarker` and `{Trait}GroupMember`
    /// traits.
    ///
    /// ```ignore
    /// trait Meta { type A; type B; }
    /// // ->
    /// trait Meta: ::groupoid::State {
    ///     type A;
    ///     type B;
    ///     type __GroupoidLayoutA;
    ///     type __GroupoidLayoutB;
    ///     type Marker: MetaGroupMarker<A = Self::A, B = Self::B>;
    /// }
    /// trait MetaGroupMarker { type A; type B; }
    /// trait MetaGroupMember<G: MetaGroupMarker>:
    ///     Meta<Marker = G, A = G::A, B = G::B> {}
    /// ```
    pub fn create_group_marker(&self) -> syn::Result<TokenStream> {
        let vis = &self.inner.vis;
        let trait_ident = &self.inner.ident;
        let marker_name = crate::naming::group_marker_ident(trait_ident);
        let member_name = crate::naming::group_member_ident(trait_ident);
        let group = crate::naming::group_param_ident();
        let types = &self.types;
        let assocs: Vec<_> = types.iter().map(|ty| &ty.ident).collect();

        let mut original = self.inner.clone();
        original
            .items
            .extend(assocs.iter().map(|assoc| -> TraitItem {
                let layout = crate::naming::layout_assoc_ident(assoc);
                parse_quote! {
                    #[doc(hidden)]
                    type #layout: ::groupoid::TypeLayout;
                }
            }));
        original.items.push(parse_quote! {
            type Marker: #marker_name<#(#assocs = Self::#assocs),*>;
        });
        original.supertraits.push(parse_quote!(::groupoid::State));

        let member_bound = quote! {
            #trait_ident<Marker = #group, #(#assocs = #group::#assocs),*>
        };

        Ok(quote! {
            #original

            #vis trait #marker_name {
                #(#types)*
            }

            #vis trait #member_name<#group: #marker_name>: #member_bound {}

            impl<#group: #marker_name, T: #member_bound> #member_name<#group> for T {}
        })
    }
}
