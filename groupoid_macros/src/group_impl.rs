use proc_macro2::TokenStream;
use syn::{
    AngleBracketedGenericArguments, Ident, ItemImpl, PathArguments, PathSegment, parse_quote,
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

    pub fn create_group_impl(&self) -> syn::Result<TokenStream> {
        let mut modified: ItemImpl = self.inner_impl.clone();

        let (trait_path, _) = modified.trait_.as_mut().ok_or(syn::Error::new_spanned(
            self.inner_impl,
            "Expected trait impl block, found regular.",
        ))?;

        // Rename the last segment to `{Trait}Helper<GroupName>`
        if trait_path.segments.is_empty() {
            return Err(syn::Error::new_spanned(
                &*trait_path,
                "Expected a non-empty trait path",
            ));
        }
        let last: &mut PathSegment = trait_path.segments.last_mut().unwrap();

        let helper_mod_ident = crate::naming::helper_mod_ident(&last.ident);

        last.ident = crate::naming::helper_trait_ident(&last.ident);
        let group_name = self.group_name;
        let args: AngleBracketedGenericArguments = parse_quote!(<#group_name>);
        last.arguments = PathArguments::AngleBracketed(args);

        // Insert the mod name as a prefix for the helper trait.
        let mod_index = trait_path.segments.len() - 1;
        trait_path
            .segments
            .insert(mod_index, PathSegment::from(helper_mod_ident));

        Ok(quote::quote!(#modified))
    }
}
