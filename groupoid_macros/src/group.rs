use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, ImplItem, ImplItemType, ItemImpl, LitInt, Type};

pub struct Group<'ast> {
    inner_impl: &'ast ItemImpl,
    group_name: &'ast Ident,
}

impl<'ast> Group<'ast> {
    pub fn new(
        item_impl: &'ast ItemImpl,
        group_name: &'ast Ident,
    ) -> Group<'ast> {
        Group {
            inner_impl: item_impl,
            group_name,
        }
    }

    pub fn generate_group_impl(&self) -> syn::Result<TokenStream> {
        let trait_name = self.trait_name()?;
        let states = self.states()?;

        let mut items: Vec<ImplItem> = self.inner_impl.items.clone();
        let group_impls = self.sized_group_impls(&mut items)?;

        let items_tokens = quote! { #(#items)* };
        let types = items
            .iter()
            .filter(|item| matches!(item, ImplItem::Type(_)));
        let marker_trait = crate::naming::group_marker_ident(trait_name);
        let group_name = self.group_name;

        Ok(quote! {
            pub struct #group_name;

            impl ::groupoid::Group for #group_name {}

            #group_impls

            impl #marker_trait for #group_name {
                #(#types)*
            }

            #(
                impl #trait_name for #states {
                    #items_tokens
                    type Marker = #group_name;
                }
            )*
        })
    }

    /// The implemented trait, e.g. `Testing` in
    /// `impl Testing for (StateA, StateB)`.
    fn trait_name(&self) -> syn::Result<&'ast Ident> {
        let (trait_path, _) =
            self.inner_impl.trait_.as_ref().ok_or_else(|| {
                syn::Error::new_spanned(
                    self.inner_impl,
                    "use `#[group]` on a trait impl, such as `impl \
                     Template for (StateA, StateB)`",
                )
            })?;

        trait_path.get_ident().ok_or_else(|| {
            syn::Error::new_spanned(
                trait_path,
                "name the trait with a single identifier, and bring it \
                 into scope with `use`",
            )
        })
    }

    /// The states in the `Self` tuple.
    ///
    /// `(StateA, StateB) -> [StateA, StateB]`
    fn states(&self) -> syn::Result<Vec<&'ast Ident>> {
        let Type::Tuple(tup) = self.inner_impl.self_ty.as_ref() else {
            return Err(syn::Error::new_spanned(
                &self.inner_impl.self_ty,
                "implement the trait for a tuple of states, such as \
                 `(StateA, StateB)`",
            ));
        };

        tup.elems
            .iter()
            .map(|e| {
                match e {
                    Type::Path(p) => p.path.get_ident(),
                    _ => None,
                }
                .ok_or_else(|| {
                    syn::Error::new_spanned(
                        e,
                        "name each state with a single identifier, such \
                         as `(StateA, StateB)`",
                    )
                })
            })
            .collect()
    }

    /// The size assertion and `SizedGroup`/`AlignedGroup` impls, or
    /// nothing when the associated type has no `#[size(N)]`. Strips the
    /// attribute from `items`.
    fn sized_group_impls(
        &self,
        items: &mut [ImplItem],
    ) -> syn::Result<TokenStream> {
        let Some(impl_ty) = items
            .iter_mut()
            .find_map(|item| match item {
                ImplItem::Type(impl_ty) => Some(impl_ty),
                _ => None,
            })
            .filter(|impl_ty| !impl_ty.attrs.is_empty())
        else {
            return Ok(quote! {});
        };

        let SizeAssert { assert, size, ty } =
            SizeAssert::try_from(impl_ty)?;

        let group_name = self.group_name;

        Ok(quote! {
            #assert
            impl ::groupoid::SizedGroup<#size> for #group_name {}
            impl ::groupoid::AlignedGroup<{ ::core::mem::align_of::<#ty>() }> for #group_name {}
        })
    }
}

/// A compile-time assertion that `ty` is `size` bytes, read from
/// `#[size(N)]`.
pub struct SizeAssert<'a> {
    pub assert: TokenStream,
    pub size: LitInt,
    pub ty: &'a Type,
}

impl<'a> TryFrom<&'a mut ImplItemType> for SizeAssert<'a> {
    type Error = syn::Error;

    /// Reads and removes the `#[size(N)]` on an associated type, rejecting
    /// any other attribute.
    fn try_from(impl_ty: &'a mut ImplItemType) -> syn::Result<Self> {
        let ident = &impl_ty.ident;

        let [attr] = impl_ty.attrs.as_slice() else {
            return Err(syn::Error::new(
                ident.span(),
                format!(
                    "keep one attribute, `#[size(N)]`, on associated \
                     type `{ident}` inside `#[group]`"
                ),
            ));
        };

        if !attr.path().is_ident("size") {
            return Err(syn::Error::new_spanned(
                attr,
                format!(
                    "remove this attribute: `#[group]` allows only \
                     `#[size(N)]` on associated type `{ident}`"
                ),
            ));
        }

        let size: LitInt = attr.parse_args()?;

        let ty = &impl_ty.ty;
        let msg = format!(
            "change `#[size({size})]` on `{ident}` to the size of `{}`",
            quote!(#ty),
        );

        impl_ty.attrs.clear();

        Ok(SizeAssert {
            assert: quote! {
                const _: () = assert!(::core::mem::size_of::<#ty>() == #size, #msg);
            },
            size,
            ty,
        })
    }
}
