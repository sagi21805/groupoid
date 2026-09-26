use extend::ext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    FnArg, Ident, ItemTrait, Pat, PatIdent, PatType, Signature, Token,
    TraitItem, TraitItemConst, TraitItemFn, TypePath,
    parse::{Parse, ParseStream},
};

pub struct GroupTrait<'ast> {
    args: &'ast GroupTraitArgs,
    item_trait: &'ast ItemTrait,
    marker_trait: TypePath,
    /// `<T::State as Template>::Marker`
    marker: TokenStream,
    helper_ident: Ident,
    helper_mod_ident: Ident,
}

impl<'ast> GroupTrait<'ast> {
    pub fn new(
        args: &'ast GroupTraitArgs,
        item_trait: &'ast ItemTrait,
    ) -> GroupTrait<'ast> {
        let mut marker_trait = args.ty.clone();
        if let Some(last) = marker_trait.path.segments.last_mut() {
            last.ident = crate::naming::group_marker_ident(&last.ident);
        }

        let template = &args.ty;

        GroupTrait {
            args,
            item_trait,
            marker_trait,
            marker: quote!(<T::State as #template>::Marker),
            helper_ident: crate::naming::helper_trait_ident(
                &item_trait.ident,
            ),
            helper_mod_ident: crate::naming::helper_mod_ident(
                &item_trait.ident,
            ),
        }
    }

    /// The trait, its hidden helper trait with the default bodies, and a
    /// blanket impl that
    /// forwards every method to the helper impl of the state's group.
    pub fn generate_group_trait(&self) -> syn::Result<TokenStream> {
        let GroupTrait {
            marker_trait,
            marker,
            helper_ident,
            helper_mod_ident,
            ..
        } = self;

        let ItemTrait {
            attrs,
            vis,
            unsafety,
            ident: trait_ident,
            generics,
            colon_token,
            supertraits,
            items,
            ..
        } = self.item_trait;

        if !generics.params.is_empty() {
            return Err(syn::Error::new_spanned(
                generics,
                "remove the generic parameters: `#[group_trait]` doesn't \
                 support them yet",
            ));
        }

        let declarations = items
            .iter()
            .map(|item| match item {
                TraitItem::Fn(TraitItemFn { attrs, sig, .. }) => {
                    Ok(quote!(#(#attrs)* #sig;))
                }
                TraitItem::Type(_)
                | TraitItem::Const(TraitItemConst {
                    default: None, ..
                }) => Err(syn::Error::new_spanned(
                    item,
                    "remove this item: a `#[group_trait]` trait can hold \
                     only methods and constants with a value",
                )),
                other => Ok(quote!(#other)),
            })
            .collect::<syn::Result<Vec<_>>>()?;

        let delegations = items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Fn(method) => Some(self.delegate(method)),
                _ => None,
            })
            .collect::<syn::Result<Vec<_>>>()?;

        let template = &self.args.ty;

        Ok(quote! {
            #(#attrs)*
            #unsafety #vis trait #trait_ident #colon_token #supertraits {
                #(#declarations)*
            }

            #[doc(hidden)]
            #vis mod #helper_mod_ident {
                use super::*;

                #(#attrs)*
                pub trait #helper_ident<Marker: #marker_trait> {
                    #(#items)*
                }
            }

            impl<T> #trait_ident for T
            where
                T: ::groupoid::WithState,
                T::State: #template,
                T: #helper_mod_ident::#helper_ident<#marker>,
            {
                #(#delegations)*
            }
        })
    }

    /// `method` with a body that calls the helper trait's version.
    fn delegate(&self, method: &TraitItemFn) -> syn::Result<TokenStream> {
        let sig = &method.sig;
        let args = sig.forwarded_args()?;

        let GroupTrait {
            marker,
            helper_ident,
            helper_mod_ident,
            ..
        } = self;
        let method_ident = &sig.ident;

        Ok(quote! {
            #sig {
                <T as #helper_mod_ident::#helper_ident<#marker>>::#method_ident(#(#args),*)
            }
        })
    }
}

/// `#[group_trait]`'s arguments: `by = <Template>`.
pub struct GroupTraitArgs {
    ty: TypePath,
}

impl Parse for GroupTraitArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::by>()?;
        input.parse::<Token![=]>()?;

        Ok(GroupTraitArgs { ty: input.parse()? })
    }
}

mod kw {
    syn::custom_keyword!(by);
}

#[ext]
impl Signature {
    /// The arguments to forward to the helper trait: `self` and each
    /// argument's identifier.
    fn forwarded_args(&self) -> syn::Result<Vec<TokenStream>> {
        if let Some(asyncness) = &self.asyncness {
            return Err(syn::Error::new_spanned(
                asyncness,
                "remove `async`: `#[group_trait]` doesn't support `async \
                 fn` yet",
            ));
        }

        if self.receiver().is_none() {
            return Err(syn::Error::new_spanned(
                self,
                format!(
                    "add a `self` receiver to `{}`: every \
                     `#[group_trait]` method needs one",
                    self.ident
                ),
            ));
        }

        self.inputs
            .iter()
            .enumerate()
            .map(|(index, fn_arg)| match fn_arg {
                FnArg::Receiver(_) => Ok(quote!(self)),
                FnArg::Typed(PatType { pat, .. }) => match &**pat {
                    Pat::Ident(PatIdent { ident, .. }) => {
                        Ok(quote!(#ident))
                    }
                    other => Err(syn::Error::new_spanned(
                        other,
                        format!(
                            "bind argument {index} of `{}` to a plain \
                             name, such as `value: T`: `#[group_trait]` \
                             forwards arguments by name",
                            self.ident
                        ),
                    )),
                },
            })
            .collect()
    }
}
