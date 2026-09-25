use extend::ext;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, Ident, ItemTrait, Pat, PatIdent, PatType, Signature, Token,
    TraitItem, TraitItemFn, TypePath,
    parse::{Parse, ParseStream},
};

pub struct GroupTrait<'ast> {
    args: &'ast GroupTraitArgs,
    item_trait: &'ast ItemTrait,
    marker_trait: TypePath,
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
            last.ident = format_ident!("{}GroupMarker", last.ident);
        }

        GroupTrait {
            args,
            item_trait,
            marker_trait,
            helper_ident: crate::naming::helper_trait_ident(
                &item_trait.ident,
            ),
            helper_mod_ident: crate::naming::helper_mod_ident(
                &item_trait.ident,
            ),
        }
    }

    pub fn generate_group_trait(&self) -> syn::Result<TokenStream> {
        let GroupTrait {
            marker_trait,
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

        // All the function declarations of the trait.
        let declarations: Vec<TokenStream> = items
            .iter()
            .map(|item| match item {
                TraitItem::Fn(TraitItemFn { attrs, sig, .. }) => {
                    quote!(#(#attrs)* #sig;)
                }
                other => quote!(#other),
            })
            .collect();

        // All the delegated function for the helper trait.
        let delegations = items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Fn(method) => Some(self.delegate(method)),
                _ => None,
            })
            .collect::<syn::Result<Vec<_>>>()?;

        let blueprint = &self.args.ty;
        let concrete_marker = self.marker();

        Ok(quote! {
            #(#attrs)*
            #unsafety #vis trait #trait_ident #colon_token #supertraits {
                #(#declarations)*
            }

            #[doc(hidden)]
            #[allow(non_snake_case)]
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
                T: #helper_mod_ident::#helper_ident<#concrete_marker>,
            {
                #(#delegations)*
            }
        })
    }

    /// `<T::State as `blueprint`>::Marker`
    fn marker(&self) -> TokenStream {
        let blueprint = &self.args.ty;
        quote!(<T::State as #blueprint>::Marker)
    }

    /// Change original trait function to call the helper trait impl
    /// instead.
    fn delegate(&self, method: &TraitItemFn) -> syn::Result<TokenStream> {
        let sig = &method.sig;
        let args = sig.forwarded_args()?;

        let GroupTrait {
            helper_ident,
            helper_mod_ident,
            ..
        } = self;
        let marker = self.marker();
        let method_ident = &sig.ident;

        Ok(quote! {
            #sig {
                <T as #helper_mod_ident::#helper_ident<#marker>>::#method_ident(#(#args),*)
            }
        })
    }
}

/// Parsed `#[group_trait(...)]` arguments.
pub struct GroupTraitArgs {
    _by: kw::by,
    _eq: Token![=],
    ty: TypePath,
}

impl Parse for GroupTraitArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(GroupTraitArgs {
            _by: input.parse()?,
            _eq: input.parse()?,
            ty: input.parse()?,
        })
    }
}

mod kw {
    syn::custom_keyword!(by);
}

#[ext]
impl Signature {
    /// The forwarded arguments for helper trait impl.
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
