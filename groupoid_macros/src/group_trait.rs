use quote::{format_ident, quote};
use syn::{
    FnArg, ItemTrait, Pat, Token, TraitItem, TypePath,
    parse::{Parse, ParseStream},
};

mod kw {
    syn::custom_keyword!(by);
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

pub struct GroupTrait<'ast> {
    args: &'ast GroupTraitArgs,
    item_trait: &'ast ItemTrait,
}

impl<'ast> GroupTrait<'ast> {
    pub fn new(args: &'ast GroupTraitArgs, item_trait: &'ast ItemTrait) -> GroupTrait<'ast> {
        GroupTrait { args, item_trait }
    }

    pub fn generate_group_trait(&self) -> syn::Result<proc_macro2::TokenStream> {
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
                &generics,
                "#[group_trait] does not currently support generic parameters \
                 on the trait itself",
            ));
        }

        let mut marker_trait = self.args.ty.clone();
        if let Some(last) = marker_trait.path.segments.last_mut() {
            last.ident = format_ident!("{}GroupMarker", last.ident);
        }

        let has_state_trait = quote!(::groupoid::WithState);
        let metadata_trait = &self.args.ty;

        // Fixed associated-type names, matching the example this macro is based on.
        let state_assoc = format_ident!("State");
        let marker_assoc = format_ident!("Marker");

        let helper_ident = format_ident!("{}Helper", trait_ident);

        // `<T::State as Metadata>::Marker`, the concrete marker type used to
        // pick which `AHelper` impl applies to a given `T`.
        let concrete_marker = quote!(<T::#state_assoc as #metadata_trait>::#marker_assoc);

        let mut trait_items = Vec::with_capacity(items.len());
        let mut helper_items = Vec::with_capacity(items.len());
        let mut delegated = Vec::with_capacity(items.len());

        for item in items {
            match item.clone() {
                TraitItem::Fn(mut method) => {
                    // Both the public trait and the helper trait declare the
                    // signature only; the real body lives in the blanket impl
                    // (for the public trait) and in each marker-specific impl
                    // (for the helper trait).
                    method.default = None;
                    method.semi_token = Some(Default::default());

                    let sig = method.sig.clone();
                    let method_ident = sig.ident.clone();
                    let is_async = sig.asyncness.is_some();

                    let mut call_args = Vec::new();
                    for (idx, fn_arg) in sig.inputs.iter().enumerate() {
                        if let FnArg::Typed(pat_type) = fn_arg {
                            match &*pat_type.pat {
                                Pat::Ident(pat_ident) => {
                                    let id = &pat_ident.ident;
                                    call_args.push(quote!(#id));
                                }
                                other_pat => {
                                    return Err(syn::Error::new_spanned(
                                        other_pat,
                                        format!(
                                            "argument {idx} of `{method_ident}` must be a \
                                             simple identifier for #[group_trait] to \
                                             forward it automatically"
                                        ),
                                    ));
                                }
                            }
                        }
                    }

                    let maybe_await = if is_async { quote!(.await) } else { quote!() };

                    delegated.push(quote! {
                        #sig {
                            <T as #helper_ident<#concrete_marker>>::#method_ident(
                                self #(, #call_args)*
                            ) #maybe_await
                        }
                    });

                    trait_items.push(TraitItem::Fn(method.clone()));
                    helper_items.push(TraitItem::Fn(method));
                }
                other => {
                    // Consts / associated types are passed through on both
                    // trait declarations verbatim; they are not auto-delegated.
                    trait_items.push(other.clone());
                    helper_items.push(other);
                }
            }
        }

        let colon = colon_token.map(|c| quote!(#c #supertraits));

        let expanded = quote! {
            #(#attrs)*
            #unsafety #vis trait #trait_ident #colon {
                #(#trait_items)*
            }

            #(#attrs)*
            #vis trait #helper_ident<Marker: #marker_trait> {
                #(#helper_items)*
            }

            impl<T> #trait_ident for T
            where
                T: #has_state_trait,
                T::#state_assoc: #metadata_trait,
                T: #helper_ident<#concrete_marker>,
            {
                #(#delegated)*
            }
        };

        Ok(expanded)
    }
}
