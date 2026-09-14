use extend::ext;
use proc_macro2::TokenStream;
use syn::{
    Fields, GenericParam, Generics, Ident, ItemStruct, PathArguments, Token, Type, TypeParam,
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
    /// the struct's sole generic type parameter.
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

pub(crate) struct TypeStateArg {
    /// The original struct with `#[repr(C)]` applied and the
    /// state parameter's changed to include `::groupoid::State`.
    item_struct: ItemStruct,
    /// The generic type parameter carrying the state.
    state_ident: Ident,
    /// Whether the struct's single field is a state projection,
    has_state_projected_field: bool,
}

impl TypeStateArg {
    pub(crate) fn new(state: TypeState, mut item_struct: ItemStruct) -> syn::Result<Self> {
        let has_state_projected_field = item_struct.has_single_state_projected_field(&state.ident);

        item_struct.ensure_repr_c();
        item_struct
            .state_type_param(&state.ident)?
            .bounds
            .push(parse_quote!(::groupoid::State));

        Ok(TypeStateArg {
            item_struct,
            state_ident: state.ident,
            has_state_projected_field,
        })
    }

    pub(crate) fn generate_has_state_impl(&self) -> TokenStream {
        let item_struct = &self.item_struct;
        let with_state_impl = self.with_state_impl();

        // Whatever bound on the state param grants the field's projection
        // (required for the field to type-check in the first place) is assumed
        // to also grant `Marker`; if it turns out not to, rustc rejects the
        // generated impls with an ordinary associated-type error.
        //
        // `TransmutableState` links `Self` to *every other* state that could be
        // plugged into the same struct, at the same `SizedGroup` size, so it is
        // meaningful under the exact same condition as `SizedWithState`.
        let sized_with_state_impl = self
            .has_state_projected_field
            .then(|| self.sized_with_state_impl());
        let transmutable_state_impl = self
            .has_state_projected_field
            .then(|| self.transmutable_state_impl());

        quote! {
            #item_struct

            #with_state_impl

            #sized_with_state_impl

            #transmutable_state_impl
        }
    }

    fn with_state_impl(&self) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let state_ident = &self.state_ident;
        // split_for_impl() correctly handles any number of generics + where clauses
        let (impl_generics, ty_generics, where_clause) = self.item_struct.generics.split_for_impl();

        quote! {
            impl #impl_generics ::groupoid::WithState for #struct_ident #ty_generics #where_clause {
                type State = #state_ident;
            }
        }
    }

    // Implement `SizedWithState<N>` if the type of the blueprint inside the struct
    // is #[size(N)]
    fn sized_with_state_impl(&self) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let (_, ty_generics, _) = self.item_struct.generics.split_for_impl();

        let mut sized_generics = self.item_struct.generics.clone();
        sized_generics.pin_states_to_size([&self.state_ident]);

        let (sized_impl_generics, _, sized_where_clause) = sized_generics.split_for_impl();

        quote! {
            impl #sized_impl_generics ::groupoid::SizedWithState<__GROUPOID_SIZE>
                for #struct_ident #ty_generics #sized_where_clause {}
        }
    }

    /// Implement `TransmutableState<Struct<S2>, N>` for `Struct<S>`, generic
    /// over *every* other state `S2` that could be plugged in (given the same
    /// bounds as `S`) with the same N constant
    ///
    /// # Example
    ///
    /// Given the `Wrap` struct from `tests/transmute_state.rs`:
    ///
    /// ```ignore
    /// #[typestate(state = S)]
    /// struct Wrap<S: Meta> {
    ///     value: S::Value,
    /// }
    /// ```
    ///
    /// this method contributes the following to the expansion
    /// (`cargo expand --package groupoid_macros --test transmute_state`):
    ///
    /// ```ignore
    /// unsafe impl<
    ///     S: Meta + ::groupoid::State,
    ///     __GroupoidToState: Meta + ::groupoid::State,
    ///     const __GROUPOID_SIZE: usize,
    /// > ::groupoid::TransmutableState<Wrap<__GroupoidToState>, __GROUPOID_SIZE> for Wrap<S>
    /// where
    ///     S::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>,
    ///     __GroupoidToState::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>,
    /// {}
    /// ```
    ///
    /// In that test `Small::Marker = SmallGroup: SizedGroup<4>` and
    /// `Big::Marker = BigGroup: SizedGroup<4>`, so both where-predicates are
    /// satisfiable only with `__GROUPOID_SIZE == 4` - which is what lets
    /// `Wrap<Small>` transmute into `Wrap<Big>`, and what stops it from
    /// transmuting into a state whose `Value` is a different size.
    fn transmutable_state_impl(&self) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let to_state_ident = format_ident!("__GroupoidToState");

        // Renaming the state param inside a clone of the struct's generics
        // produces both halves of the `To` side at once: the `Wrap<__GroupoidToState>`
        // type arguments below, and a `__GroupoidToState` parameter that already
        // carries `S`'s bounds (`Meta + ::groupoid::State`).
        let mut to_generics = self.item_struct.generics.clone();
        let to_state_param = {
            let param = to_generics
                .state_param_mut(&self.state_ident)
                .expect("the state ident was checked to be a type param in `new`");
            param.ident = to_state_ident.clone();
            param.clone()
        };

        // The impl header needs both states in scope at once, hence the
        // `<S, __GroupoidToState, ..>` parameter list.
        let mut impl_generics_source = self.item_struct.generics.clone();
        impl_generics_source
            .params
            .push(GenericParam::Type(to_state_param));

        // Both states are then pinned to the *same* `__GROUPOID_SIZE`, which is
        // the safety argument for the `unsafe impl` spelled out in full below.
        impl_generics_source.pin_states_to_size([&self.state_ident, &to_state_ident]);

        let (impl_generics, _, where_clause) = impl_generics_source.split_for_impl();
        // `Wrap<__GroupoidToState>` (the `To` type argument) ...
        let (_, to_ty_generics, _) = to_generics.split_for_impl();
        // ... and `Wrap<S>` (the `for` type).
        let (_, self_ty_generics, _) = self.item_struct.generics.split_for_impl();

        quote! {
            // SAFETY: `Self` and `To` are literally the same struct with
            // only the state parameter swapped, and both sides' state
            // values are pinned to the same `SizedGroup` size above.
            unsafe impl #impl_generics ::groupoid::TransmutableState<
                #struct_ident #to_ty_generics, __GROUPOID_SIZE
            > for #struct_ident #self_ty_generics #where_clause {}
        }
    }
}

// The rest of this module is plain syntax manipulation on `syn`'s own types,
// hung off those types as extension traits so it reads as
// `item_struct.state_type_param(..)` rather than as functions taking the thing
// they act on. None of it belongs on `TypeStateArg`: each one either runs
// during construction, before there is a `self` to speak of, or edits a
// throwaway clone of some generics rather than anything the type owns.

#[ext]
impl ItemStruct {
    /// `#[repr(C)]` is useful so no matter what, the state field will always be
    /// in the same place. An explicit `repr` from the author wins.
    fn ensure_repr_c(&mut self) {
        let has_repr = self.attrs.iter().any(|attr| attr.path().is_ident("repr"));
        if !has_repr {
            self.attrs.push(parse_quote!(#[repr(C)]));
        }
    }

    /// Looks up `state_ident` among the struct's generic type parameters,
    /// erroring if it isn't one of them.
    fn state_type_param(&mut self, state_ident: &Ident) -> syn::Result<&mut TypeParam> {
        let msg = format!(
            "`{}` must be one of the generic type parameters of `{}`",
            state_ident, self.ident
        );
        self.generics
            .state_param_mut(state_ident)
            .ok_or_else(|| syn::Error::new(state_ident.span(), msg))
    }

    /// Whether the struct has exactly one field and that field is a projection
    /// through the state, like `S::Value`. Only then do `SizedWithState` and
    /// `TransmutableState` mean anything.
    fn has_single_state_projected_field(&self, state_ident: &Ident) -> bool {
        let field = match &self.fields {
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

#[ext(name = GenericsExt)]
impl Generics {
    /// The type parameter named `state_ident`, if these generics declare one.
    fn state_param_mut(&mut self, state_ident: &Ident) -> Option<&mut TypeParam> {
        self.params.iter_mut().find_map(|p| match p {
            GenericParam::Type(tp) if tp.ident == *state_ident => Some(tp),
            _ => None,
        })
    }

    /// Adds the `const __GROUPOID_SIZE: usize` parameter shared by
    /// `SizedWithState` and `TransmutableState`, plus one
    /// `<State>::Marker: SizedGroup<__GROUPOID_SIZE>` predicate per state.
    ///
    /// Each state's `Marker` implements `SizedGroup<N>` for exactly the one `N`
    /// its `#[size(N)]` declared, so listing several states here forces them
    /// all onto the same `N` - that is what makes the size equality a type
    /// error rather than an assumption. `states` is spelled out by the caller
    /// because the predicates land in that order, and that order is part of the
    /// generated output.
    fn pin_states_to_size<'a>(&mut self, states: impl IntoIterator<Item = &'a Ident>) {
        self.params.push(parse_quote!(const __GROUPOID_SIZE: usize));

        let predicates = &mut self.make_where_clause().predicates;
        for state in states {
            predicates.push(parse_quote!(
                #state::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>
            ));
        }
    }
}
