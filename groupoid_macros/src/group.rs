use proc_macro2::TokenStream;
use quote::{format_ident, quote};
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
        let marker_trait = format_ident!("{}GroupMarker", trait_name);
        let group_name = self.group_name;

        Ok(quote! {
            struct #group_name;

            impl ::groupoid::Group for #group_name {}

            #group_impls

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

    /// The trait this `#[group]` impl block is implementing, e.g.
    /// `Testing` in `impl Testing for (StateA, StateB) { .. }`.
    fn trait_name(&self) -> syn::Result<&'ast Ident> {
        let (trait_path, _) = self.inner_impl.trait_.as_ref().ok_or(
            syn::Error::new_spanned(
                self.inner_impl,
                "Expected trait impl block, found regular.",
            ),
        )?;

        trait_path.get_ident().ok_or(syn::Error::new_spanned(
            trait_path,
            "Expected trait path name to be single ident",
        ))
    }

    /// The states this group applies to, parsed out of the tuple `Self`
    /// type, e.g. `[StateA, StateB]` from `impl Testing for (StateA,
    /// StateB)`.
    fn states(&self) -> syn::Result<Vec<&'ast Ident>> {
        let Type::Tuple(tup) = self.inner_impl.self_ty.as_ref() else {
            return Err(syn::Error::new_spanned(
                self.inner_impl,
                "Expected tuple for the group states, e.g. (StateA, \
                 StateB ...)",
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
                        "Expected the types inside the group tuple to be \
                         single idents, e.g. (StateA, StateB ...)",
                    )
                })
            })
            .collect()
    }

    /// The size assertion and `SizedGroup`/`AlignedGroup` impls for the
    /// group's associated type, or an empty stream if it carries no
    /// `#[size(N)]`.
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

/// What a `#[size(N)]` on an associated type established: the
/// compile-time assertion that `ty` really is `size` bytes, together with
/// the two facts the trait impls are built from.
pub struct SizeAssert<'a> {
    pub assert: TokenStream,
    pub size: LitInt,
    pub ty: &'a Type,
}

impl<'a> TryFrom<&'a mut ImplItemType> for SizeAssert<'a> {
    type Error = syn::Error;

    /// Reads the `#[size(N)]` off an associated type, rejecting any other
    /// attribute, and returns the assertion it stands for. The attribute
    /// is left in place; stripping it is the caller's job.
    fn try_from(impl_ty: &'a mut ImplItemType) -> syn::Result<Self> {
        let ident = &impl_ty.ident;

        let [attr] = impl_ty.attrs.as_slice() else {
            return Err(syn::Error::new(
                ident.span(),
                format!(
                    "exactly one `#[size(N)]` attribute is expected on \
                     associated type `{}` inside `#[group]`, found {} \
                     attribute(s)",
                    ident,
                    impl_ty.attrs.len()
                ),
            ));
        };

        if !attr.path().is_ident("size") {
            return Err(syn::Error::new_spanned(
                attr,
                format!(
                    "only `#[size(N)]` is allowed on associated type \
                     `{}` inside `#[group]`",
                    ident
                ),
            ));
        }

        let size: LitInt = attr.parse_args()?;

        let ty = &impl_ty.ty;
        let msg = format!(
            "associated type `{}` is `{}`, which is not {} byte(s)",
            ident,
            quote!(#ty).to_string(),
            size
        );

        // Remove the #[size(N)] attr from the generated tokenstream
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
