use proc_macro2::TokenStream;
use syn::{
    GenericParam, Ident, ItemStruct, Token,
    parse::{Parse, ParseStream},
};

use quote::quote;

/// Parses `state = SomeIdent`
pub(crate) struct State {
    ident: Ident,
}

impl Parse for State {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "state" {
            return Err(syn::Error::new(key.span(), "expected `state = <Ident>`"));
        }
        input.parse::<Token![=]>()?;
        let ident: Ident = input.parse()?;
        Ok(State { ident })
    }
}

pub(crate) struct StateArg<'ast> {
    state: &'ast State,
    item_struct: &'ast ItemStruct,
}

impl<'ast> StateArg<'ast> {
    pub(crate) fn new(state: &'ast State, item_struct: &'ast ItemStruct) -> StateArg<'ast> {
        StateArg { state, item_struct }
    }

    pub(crate) fn generate_has_state_impl(&self) -> syn::Result<TokenStream> {
        // Ensure the given ident is actually one of the struct's generic type params.
        let is_valid_generic = self
            .item_struct
            .generics
            .params
            .iter()
            .any(|p| matches!(p, GenericParam::Type(tp) if tp.ident == self.state.ident));

        if !is_valid_generic {
            let msg = format!(
                "`{}` must be one of the generic type parameters of `{}`",
                self.state.ident, self.item_struct.ident
            );
            return Err(syn::Error::new(self.state.ident.span(), msg));
        }

        let struct_ident = &self.item_struct.ident;
        // split_for_impl() correctly handles any number of generics + where clauses
        let (impl_generics, ty_generics, where_clause) = self.item_struct.generics.split_for_impl();
        let item_struct = &self.item_struct;
        let state_ident = &self.state.ident;

        let expanded = quote! {
            #item_struct

            impl #impl_generics HasState for #struct_ident #ty_generics #where_clause {
                type State = #state_ident;
            }
        };

        Ok(expanded.into())
    }
}
