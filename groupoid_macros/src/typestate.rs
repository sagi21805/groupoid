use proc_macro2::TokenStream;
use syn::{
    GenericParam, Ident, ItemStruct, Token,
    parse::{Parse, ParseStream},
    parse_quote,
};

use quote::quote;

/// Parses `state = SomeIdent`
pub(crate) struct TypeState {
    ident: Ident,
}

impl Parse for TypeState {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "state" {
            return Err(syn::Error::new(key.span(), "expected `state = <Ident>`"));
        }
        input.parse::<Token![=]>()?;
        let ident: Ident = input.parse()?;
        Ok(TypeState { ident })
    }
}

pub(crate) struct TypeStateArg<'ast> {
    state: &'ast TypeState,
    item_struct: &'ast ItemStruct,
}

impl<'ast> TypeStateArg<'ast> {
    pub(crate) fn new(state: &'ast TypeState, item_struct: &'ast ItemStruct) -> TypeStateArg<'ast> {
        TypeStateArg { state, item_struct }
    }

    pub(crate) fn generate_has_state_impl(&self) -> syn::Result<TokenStream> {
        let state_ident = &self.state.ident;

        // Ensure the given ident is actually one of the struct's generic type params,
        // and add a `::groupoid::State` bound to it so callers don't have to spell it
        // out themselves.
        let mut item_struct = self.item_struct.clone();
        let matched_param = item_struct
            .generics
            .params
            .iter_mut()
            .find(|p| matches!(p, GenericParam::Type(tp) if tp.ident == *state_ident));

        let Some(GenericParam::Type(type_param)) = matched_param else {
            let msg = format!(
                "`{}` must be one of the generic type parameters of `{}`",
                state_ident, self.item_struct.ident
            );
            return Err(syn::Error::new(state_ident.span(), msg));
        };

        type_param.bounds.push(parse_quote!(::groupoid::State));

        let struct_ident = &item_struct.ident;
        // split_for_impl() correctly handles any number of generics + where clauses
        let (impl_generics, ty_generics, where_clause) = item_struct.generics.split_for_impl();

        let expanded = quote! {
            #item_struct

            impl #impl_generics ::groupoid::WithState for #struct_ident #ty_generics #where_clause {
                type State = #state_ident;
            }
        };

        Ok(expanded.into())
    }
}
