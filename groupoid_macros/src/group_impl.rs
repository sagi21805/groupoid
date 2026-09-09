use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    GenericParam, Ident, ImplItem, ImplItemFn, ItemImpl, Signature, Type, TypeParamBound, TypePath,
    WherePredicate,
};

pub struct GroupImpl<'ast> {
    inner_impl: &'ast ItemImpl,
    group_name: &'ast Ident,
}

impl<'ast> GroupImpl<'ast> {
    pub fn new(item_impl: &'ast ItemImpl, group_name: &'ast Ident) -> GroupImpl<'ast> {
        GroupImpl {
            inner_impl: item_impl,
            group_name,
        }
    }

    fn find_state_generic(&self, trait_name: &Ident) -> syn::Result<&'ast Ident> {
        let generics = &self.inner_impl.generics;
        for param in &generics.params {
            if let GenericParam::Type(type_param) = param {
                for bound in &type_param.bounds {
                    if let TypeParamBound::Trait(trait_bound) = bound {
                        if trait_bound.path.get_ident().ok_or(syn::Error::new_spanned(
                            trait_bound,
                            "Currently not supporting traits full name",
                        ))? == trait_name
                        {
                            return Ok(&type_param.ident);
                        }
                    }
                }
            }
        }

        if let Some(where_clause) = &self.inner_impl.generics.where_clause {
            for predicate in &where_clause.predicates {
                if let WherePredicate::Type(predicate_type) = &predicate {
                    for bound in &predicate_type.bounds {
                        if let TypeParamBound::Trait(trait_bound) = bound {
                            if trait_bound.path.get_ident().ok_or(syn::Error::new_spanned(
                                trait_bound,
                                "Currently not supporting traits full name",
                            ))? == trait_name
                            {
                                if let Type::Path(path_ty) = &predicate_type.bounded_ty {
                                    return Ok(path_ty.path.get_ident().ok_or(
                                        syn::Error::new_spanned(
                                            path_ty,
                                            "Currently supporting only single ident types",
                                        ),
                                    )?);
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(syn::Error::new_spanned(
            &self.inner_impl,
            "Did not find generic parameter that satisfies the trait implementation",
        ))
    }

    pub fn create_group_impl(&self) -> syn::Result<TokenStream> {
        let (trait_path, _) = self
            .inner_impl
            .trait_
            .as_ref()
            .ok_or(syn::Error::new_spanned(
                self.inner_impl,
                "Expected trait impl block, found regular.",
            ))?;

        let trait_name = trait_path.get_ident().ok_or(syn::Error::new_spanned(
            trait_path,
            "Expected trait path name to be single ident",
        ))?;

        let helper_name = format_ident!("{}Helper", trait_name);

        let signatures = self
            .inner_impl
            .items
            .iter()
            .filter_map(|i| {
                if let ImplItem::Fn(f) = i {
                    Some(&f.sig)
                } else {
                    None
                }
            })
            .collect::<Vec<&Signature>>();

        let modified: ItemImpl = self.inner_impl.clone();

        let helper_trait = quote! {
            trait #helper_name<Marker> {
                #(#signatures;)*
            }
        };

        Ok(helper_trait)
    }
}
