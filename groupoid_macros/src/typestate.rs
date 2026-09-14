use proc_macro2::TokenStream;
use syn::{
    Fields, GenericParam, Ident, ItemStruct, PathArguments, Token, Type, TypeParam,
    parse::{Parse, ParseStream},
    parse_quote,
};

use quote::{format_ident, quote};

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

impl TypeState {
    /// `#[typestate]` used with no `state = <Ident>` argument falls back to
    /// the struct's sole generic type parameter - lifetimes and const
    /// generics don't count, and don't disambiguate. More than one type
    /// parameter, or none, means the caller has to say which one is the
    /// state explicitly.
    pub(crate) fn infer(item_struct: &ItemStruct) -> syn::Result<TypeState> {
        let type_params: Vec<_> = item_struct
            .generics
            .params
            .iter()
            .filter_map(|p| match p {
                GenericParam::Type(tp) => Some(tp),
                _ => None,
            })
            .collect();

        let [only] = type_params.as_slice() else {
            return Err(syn::Error::new_spanned(
                &item_struct.ident,
                "`#[typestate]` needs `state = <Ident>` unless the struct has exactly one generic type parameter",
            ));
        };

        Ok(TypeState {
            ident: only.ident.clone(),
        })
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
        let mut item_struct = self.item_struct.clone();

        // #[repr(C)] is useful so no matter what, the state field will always be in the
        // same place.
        Self::ensure_repr_c(&mut item_struct);

        let type_param = Self::state_type_param(&mut item_struct, state_ident)?;
        type_param.bounds.push(parse_quote!(::groupoid::State));

        let with_state_impl = Self::with_state_impl(&item_struct, state_ident);

        // See `has_single_state_projected_field` for why this shape is required.
        // Whatever bound on the state param grants that projection (required
        // for the field to type-check in the first place) is assumed to also
        // grant `Marker`; if it turns out not to, rustc rejects the generated
        // `SizedWithState` impl with an ordinary associated-type error.
        let sized_with_state_impl =
            Self::has_single_state_projected_field(&self.item_struct.fields, state_ident)
                .then(|| Self::sized_with_state_impl(&item_struct, state_ident));

        // `TransmutableState` links `Self` to *every other* state that could
        // be plugged into the same struct, at the same `SizedGroup` size, so
        // it is only meaningful under the exact same condition as
        // `SizedWithState` above.
        let transmutable_state_impl =
            Self::has_single_state_projected_field(&self.item_struct.fields, state_ident)
                .then(|| Self::transmutable_state_impl(&item_struct, state_ident));

        Ok(quote! {
            #item_struct

            #with_state_impl

            #sized_with_state_impl

            #transmutable_state_impl
        })
    }

    fn ensure_repr_c(item_struct: &mut ItemStruct) {
        let has_repr = item_struct
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("repr"));
        if !has_repr {
            item_struct.attrs.push(parse_quote!(#[repr(C)]));
        }
    }

    /// Looks up `state_ident` among `item_struct`'s generic type parameters,
    /// erroring if it isn't one of them.
    fn state_type_param<'a>(
        item_struct: &'a mut ItemStruct,
        state_ident: &Ident,
    ) -> syn::Result<&'a mut TypeParam> {
        let matched_param = item_struct
            .generics
            .params
            .iter_mut()
            .find(|p| matches!(p, GenericParam::Type(tp) if tp.ident == *state_ident));

        let Some(GenericParam::Type(type_param)) = matched_param else {
            let msg = format!(
                "`{}` must be one of the generic type parameters of `{}`",
                state_ident, item_struct.ident
            );
            return Err(syn::Error::new(state_ident.span(), msg));
        };
        Ok(type_param)
    }

    fn with_state_impl(item_struct: &ItemStruct, state_ident: &Ident) -> TokenStream {
        let struct_ident = &item_struct.ident;
        // split_for_impl() correctly handles any number of generics + where clauses
        let (impl_generics, ty_generics, where_clause) = item_struct.generics.split_for_impl();

        quote! {
            impl #impl_generics ::groupoid::WithState for #struct_ident #ty_generics #where_clause {
                type State = #state_ident;
            }
        }
    }

    // Implement `SizedWithState<N>` if the type of the blueprint inside the struct
    // is #[size(N)]
    fn sized_with_state_impl(item_struct: &ItemStruct, state_ident: &Ident) -> TokenStream {
        let struct_ident = &item_struct.ident;
        let (_, ty_generics, _) = item_struct.generics.split_for_impl();

        let mut sized_generics = item_struct.generics.clone();

        sized_generics
            .params
            .push(parse_quote!(const __GROUPOID_SIZE: usize));

        sized_generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(
                #state_ident::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>
            ));

        let (sized_impl_generics, _, sized_where_clause) = sized_generics.split_for_impl();

        quote! {
            impl #sized_impl_generics ::groupoid::SizedWithState<__GROUPOID_SIZE>
                for #struct_ident #ty_generics #sized_where_clause {}
        }
    }

    /// Implement `TransmutableState<Struct<S2>, N>` for `Struct<S>`, generic
    /// over *every* other state `S2` that could be plugged in (given the same
    /// bounds as `S`) and every `N` for which both states' blueprint values
    /// carry that `SizedGroup` size. `item_struct` is expected to already
    /// carry the `::groupoid::State` bound `#[typestate]` adds to `S`.
    fn transmutable_state_impl(item_struct: &ItemStruct, state_ident: &Ident) -> TokenStream {
        let struct_ident = &item_struct.ident;
        let to_state_ident = format_ident!("__GroupoidToState");

        // `To`'s type arguments: the same generics as `Self`, with the state
        // parameter's *use* renamed to a second, independent identifier.
        let mut to_generics = item_struct.generics.clone();
        for param in to_generics.params.iter_mut() {
            if let GenericParam::Type(tp) = param {
                if tp.ident == *state_ident {
                    tp.ident = to_state_ident.clone();
                }
            }
        }
        let (_, to_ty_generics, _) = to_generics.split_for_impl();
        let (_, self_ty_generics, _) = item_struct.generics.split_for_impl();

        // The impl needs both state identifiers in scope at once - clone the
        // original (already `State`-bounded) state param under the second
        // identifier and splice it in right after the first, so type params
        // stay grouped ahead of any const generics.
        let mut impl_generics_source = item_struct.generics.clone();
        let state_index = impl_generics_source
            .params
            .iter()
            .position(|p| matches!(p, GenericParam::Type(tp) if tp.ident == *state_ident))
            .expect("state_ident is a type param of item_struct");
        let mut to_state_param = match &impl_generics_source.params[state_index] {
            GenericParam::Type(tp) => tp.clone(),
            _ => unreachable!("state_index points at a GenericParam::Type"),
        };
        to_state_param.ident = to_state_ident.clone();
        impl_generics_source
            .params
            .insert(state_index + 1, GenericParam::Type(to_state_param));
        impl_generics_source
            .params
            .push(parse_quote!(const __GROUPOID_SIZE: usize));

        impl_generics_source
            .make_where_clause()
            .predicates
            .push(parse_quote!(
                #state_ident::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>
            ));
        impl_generics_source
            .make_where_clause()
            .predicates
            .push(parse_quote!(
                #to_state_ident::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>
            ));

        let (impl_generics, _, where_clause) = impl_generics_source.split_for_impl();

        quote! {
            // SAFETY: `Self` and `To` are literally the same struct with
            // only the state parameter swapped, and both sides' state
            // values are pinned to the same `SizedGroup` size above.
            unsafe impl #impl_generics ::groupoid::TransmutableState<
                #struct_ident #to_ty_generics, __GROUPOID_SIZE
            > for #struct_ident #self_ty_generics #where_clause {}
        }
    }

    /// Ensure that the struct has exactly one field that uses the blueprint
    /// state item.
    fn has_single_state_projected_field(fields: &Fields, state_ident: &Ident) -> bool {
        let field = match fields {
            Fields::Named(f) if f.named.len() == 1 => f.named.first(),
            Fields::Unnamed(f) if f.unnamed.len() == 1 => f.unnamed.first(),
            _ => None,
        };
        let Some(field) = field else {
            return false;
        };

        let Type::Path(type_path) = &field.ty else {
            return false;
        };
        if type_path.qself.is_some() || type_path.path.leading_colon.is_some() {
            return false;
        }
        let segments: Vec<_> = type_path.path.segments.iter().collect();
        let [state_seg, assoc_seg] = segments.as_slice() else {
            return false;
        };
        if state_seg.ident != *state_ident || state_seg.arguments != PathArguments::None {
            return false;
        }
        assoc_seg.arguments == PathArguments::None
    }
}
