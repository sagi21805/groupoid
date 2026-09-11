use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, ImplItem, ImplItemType, ItemImpl, LitInt, Type};

pub struct Group<'ast> {
    inner_impl: &'ast ItemImpl,
    group_name: &'ast Ident,
}

impl<'ast> Group<'ast> {
    pub fn new(item_impl: &'ast ItemImpl, group_name: &'ast Ident) -> Group<'ast> {
        Group {
            inner_impl: item_impl,
            group_name,
        }
    }

    pub fn generate_group_impl(&self) -> syn::Result<TokenStream> {
        let trait_name = self.trait_name()?;
        let states = self.states()?;

        let mut items: Vec<ImplItem> = self.inner_impl.items.clone();
        let size_asserts = self.size_asserts(&mut items)?;

        let items_tokens = quote! { #(#items)* };
        let marker_trait = format_ident!("{}GroupMarker", trait_name);
        let group_name = self.group_name;

        Ok(quote! {
            struct #group_name;

            impl ::groupoid::Group for #group_name {}

            #(#size_asserts)*

            impl #marker_trait for #group_name {
                #items_tokens
            }

            #(
                impl #trait_name for #states{

                    #items_tokens
                    type Marker = #group_name;
                }
            )*
        })
    }

    /// The trait this `#[group]` impl block is implementing, e.g. `Testing`
    /// in `impl Testing for (StateA, StateB) { .. }`.
    fn trait_name(&self) -> syn::Result<&'ast Ident> {
        let (trait_path, _) = self
            .inner_impl
            .trait_
            .as_ref()
            .ok_or(syn::Error::new_spanned(
                self.inner_impl,
                "Expected trait impl block, found regular.",
            ))?;

        trait_path.get_ident().ok_or(syn::Error::new_spanned(
            trait_path,
            "Expected trait path name to be single ident",
        ))
    }

    /// The states this group applies to, parsed out of the tuple `Self`
    /// type, e.g. `[StateA, StateB]` from `impl Testing for (StateA, StateB)`.
    fn states(&self) -> syn::Result<Vec<&'ast Ident>> {
        let Type::Tuple(tup) = self.inner_impl.self_ty.as_ref() else {
            return Err(syn::Error::new_spanned(
                self.inner_impl,
                "Expected tuple for the group states, e.g. (StateA, StateB ...)",
            ));
        };

        tup.elems
            .iter()
            .map(|e| match e {
                Type::Path(p) => Ok(p.path.get_ident().ok_or(syn::Error::new_spanned(
                    p,
                    "Expected the types inside the group tuple to be single ident",
                ))?),
                _ => Err(syn::Error::new_spanned(
                    e,
                    "Expected the types inside the group tuple to be path like",
                )),
            })
            .collect()
    }

    // An optional `#[size(N)]` to specify specific size for the type.
    fn size_asserts(&self, items: &mut [ImplItem]) -> syn::Result<Vec<TokenStream>> {
        items
            .iter_mut()
            .filter_map(|item| match item {
                ImplItem::Type(impl_ty) => Some(impl_ty),
                _ => None,
            })
            .map(|impl_ty| self.size_assert_for_type(impl_ty))
            .collect()
    }

    /// Strips `#[size(N)]` (rejecting any other attribute) off a single
    /// associated type, returning the compile-time size assertion and
    /// `SizedGroup` impl it implies, if the attribute was present.
    fn size_assert_for_type(&self, impl_ty: &mut ImplItemType) -> syn::Result<TokenStream> {
        let (size_attrs, other_attrs): (Vec<_>, Vec<_>) = std::mem::take(&mut impl_ty.attrs)
            .into_iter()
            .partition(|attr| attr.path().is_ident("size"));

        let ident = &impl_ty.ident;

        if let Some(attr) = other_attrs.first() {
            return Err(syn::Error::new_spanned(
                attr,
                format!(
                    "only `#[size(N)]` is allowed on an associated type inside `#[group]`, found other attribute on `{}`",
                    ident
                ),
            ));
        }

        if size_attrs.len() > 1 {
            return Err(syn::Error::new_spanned(
                &size_attrs[1],
                format!(
                    "only one `#[size(N)]` is allowed per associated type, found {} on `{}`",
                    size_attrs.len(),
                    ident
                ),
            ));
        }

        let Some(attr) = size_attrs.into_iter().next() else {
            // No size attribute, empty stream
            return Ok(quote! {});
        };

        let ty = &impl_ty.ty;
        let group_name = self.group_name;
        let expected: LitInt = attr.parse_args()?;
        let msg = format!(
            "associated type `{}` in group `{}` is `{}`, which is not {} byte(s)",
            ident,
            group_name,
            quote!(#ty).to_string(),
            expected
        );

        Ok(quote! {
            const _: () = assert!(::core::mem::size_of::<#ty>() == #expected, #msg);
            impl ::groupoid::SizedGroup<#expected> for #group_name {}
        })
    }
}
