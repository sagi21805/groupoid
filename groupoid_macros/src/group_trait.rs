use extend::ext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    FnArg, Ident, ItemTrait, Pat, PatIdent, PatType, Signature, Token,
    TraitItem, TraitItemFn, TypePath,
    parse::{Parse, ParseStream},
};

pub struct GroupTrait<'ast> {
    args: &'ast GroupTraitArgs,
    item_trait: &'ast ItemTrait,
    marker_trait: TypePath,
    /// `<T::State as Blueprint>::Marker`
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

        let blueprint = &args.ty;

        GroupTrait {
            args,
            item_trait,
            marker_trait,
            marker: quote!(<T::State as #blueprint>::Marker),
            helper_ident: crate::naming::helper_trait_ident(
                &item_trait.ident,
            ),
            helper_mod_ident: crate::naming::helper_mod_ident(
                &item_trait.ident,
            ),
        }
    }

    /// The trait, its hidden helper trait, and a blanket impl that
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
                "#[group_trait] does not currently support generic \
                 parameters on the trait itself",
            ));
        }

        let declarations: Vec<TokenStream> = items
            .iter()
            .map(|item| match item {
                TraitItem::Fn(TraitItemFn { attrs, sig, .. }) => {
                    quote!(#(#attrs)* #sig;)
                }
                other => quote!(#other),
            })
            .collect();

        let delegations = items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Fn(method) => Some(self.delegate(method)),
                _ => None,
            })
            .collect::<syn::Result<Vec<_>>>()?;

        let blueprint = &self.args.ty;

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
                    #(#declarations)*
                }
            }

            impl<T> #trait_ident for T
            where
                T: ::groupoid::WithState,
                T::State: #blueprint,
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

/// `#[group_trait]`'s arguments: `by = <Blueprint>`.
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
                "`async fn` is not supported by #[group_trait] yet",
            ));
        }

        if self.receiver().is_none() {
            return Err(syn::Error::new_spanned(
                self,
                format!(
                    "method `{}` must take `self`: #[group_trait] \
                     requires every method to have a receiver",
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
                            "argument {index} of `{}` must be a simple \
                             identifier for #[group_trait] to forward it \
                             automatically",
                            self.ident
                        ),
                    )),
                },
            })
            .collect()
    }
}
