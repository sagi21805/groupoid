use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, ItemImpl, Type};

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

    pub fn generate_group_impl(&self) -> syn::Result<TokenStream> {
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

        let Type::Tuple(tup) = self.inner_impl.self_ty.as_ref() else {
            return Err(syn::Error::new_spanned(
                self.inner_impl,
                "Expected tuple for the group states, e.g. (StateA, StateB ...)",
            ));
        };

        let states = tup
            .elems
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
            .collect::<syn::Result<Vec<_>>>()?;

        let items = &self.inner_impl.items;

        let items_tokens = quote! { #(#items)* };
        let marker_trait = format_ident!("{}GroupMarker", trait_name);
        let group_name = self.group_name;

        Ok(quote! {
            struct #group_name;

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
}
