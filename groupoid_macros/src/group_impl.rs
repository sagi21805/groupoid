use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, ItemImpl, PathArguments, PathSegment, parse_quote};

pub struct GroupImpl<'ast> {
    inner_impl: &'ast ItemImpl,
    group_name: &'ast Ident,
}

impl<'ast> GroupImpl<'ast> {
    pub fn new(
        item_impl: &'ast ItemImpl,
        group_name: &'ast Ident,
    ) -> GroupImpl<'ast> {
        GroupImpl {
            inner_impl: item_impl,
            group_name,
        }
    }

    /// The impl block retargeted at the trait's helper trait.
    ///
    /// `impl A for T -> impl __a_helper_mod::AHelper<Group> for T`
    pub fn create_group_impl(&self) -> syn::Result<TokenStream> {
        let mut modified = self.inner_impl.clone();

        let (trait_path, _) =
            modified.trait_.as_mut().ok_or_else(|| {
                syn::Error::new_spanned(
                    self.inner_impl,
                    "use `#[group_impl]` on a trait impl, such as `impl \
                     Trait for Wrap<S>`",
                )
            })?;

        let last = trait_path
            .segments
            .last_mut()
            .expect("a parsed trait path has at least one segment");

        let helper_mod_ident =
            crate::naming::helper_mod_ident(&last.ident);
        let group_name = self.group_name;
        last.ident = crate::naming::helper_trait_ident(&last.ident);
        last.arguments =
            PathArguments::AngleBracketed(parse_quote!(<#group_name>));

        let mod_index = trait_path.segments.len() - 1;
        trait_path
            .segments
            .insert(mod_index, PathSegment::from(helper_mod_ident));

        Ok(quote!(#modified))
    }
}
