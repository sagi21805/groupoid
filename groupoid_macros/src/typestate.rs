use proc_macro2::TokenStream;
use syn::{
    Fields, GenericParam, Ident, ItemStruct, PathArguments, Token, Type, TypeParam,
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

        // #[repr(C)] is useful so no matter what, the state field will always be in the same place.
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

        Ok(quote! {
            #item_struct

            #with_state_impl

            #sized_with_state_impl
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

    // Implement `SizedWithState<N>` if the type of the blueprint inside the struct is #[size(N)]
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

    /// Ensure that the struct has exactly one field that uses the blueprint state item.
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
