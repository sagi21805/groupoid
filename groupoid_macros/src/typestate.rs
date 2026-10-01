use extend::ext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, ConstParam, Field, GenericParam, Generics, Ident,
    ItemStruct, LifetimeParam, LitBool, LitInt, Path, PathArguments,
    Token, Type, TypeParam, TypePath, WherePredicate,
    parse::{Parse, ParseStream},
    parse_quote,
    visit::{self, Visit},
};

use crate::syn_ext::{
    AttributeExt as _, GenericsExt as _, OptionExt as _, TypeExt as _,
};

pub(crate) struct TypeState {
    /// The struct with the state bounded by `State`.
    item_struct: ItemStruct,
    /// The generic type parameter carrying the state.
    state: Ident,
    /// The target state, with the state's bounds.
    target_state: TypeParam,
    /// `Wrap<S> -> Wrap<__GroupoidTargetState>`
    target_ty: TokenStream,
    /// Whether `TransmutableState` is derived too.
    transmute: Transmute,
    /// The distinct associated types the fields project through the
    /// state (`Value` in `S::Value`), in order of first appearance.
    projections: Vec<Ident>,
}

impl TypeState {
    /// Resolves the state and validates the fields.
    pub(crate) fn new(
        args: TypeStateArgs,
        mut item_struct: ItemStruct,
    ) -> syn::Result<Self> {
        let state_param = item_struct.state_param(args.state.as_ref())?;
        state_param.bounds.push(parse_quote!(::groupoid::State));

        let target_state = TypeParam {
            ident: crate::naming::target_state_ident(),
            ..state_param.clone()
        };
        let state = state_param.ident.clone();
        let target_ty =
            item_struct.with_state(&state, &target_state.ident);
        let projections = item_struct.projections(&state)?;

        if let Transmute::On(align) = &args.transmute {
            item_struct.require_transmutable_layout(&state)?;
            if projections.is_empty() {
                return Err(syn::Error::new_spanned(
                    &item_struct.ident,
                    format!(
                        "add a field of type `{state}::Assoc`, or remove \
                         `unsafe_transmute = true`",
                    ),
                ));
            }
            item_struct.ensure_repr(align)?;
        }

        Ok(TypeState {
            item_struct,
            state,
            target_state,
            target_ty,
            transmute: args.transmute,
            projections,
        })
    }

    /// The struct followed by every impl `#[typestate]` derives for it.
    pub(crate) fn generate_typestate_impls(&self) -> TokenStream {
        let item_struct = &self.item_struct;
        let with_state_impl = self.with_state_impl();
        let transmutable_state_impl = match &self.transmute {
            Transmute::On(align) => {
                Some(self.transmutable_state_impl(align))
            }
            Transmute::Off => None,
        };
        let restate_impl = self.restate_impl();

        quote! {
            #item_struct

            #with_state_impl

            #restate_impl

            #transmutable_state_impl
        }
    }

    fn with_state_impl(&self) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let state = &self.state;
        let (impl_generics, ty_generics, where_clause) =
            self.item_struct.generics.split_for_impl();

        quote! {
            impl #impl_generics ::groupoid::WithState for #struct_ident #ty_generics #where_clause {
                type State = #state;
            }
        }
    }

    /// `Restate<S2>` for `Struct<S>`, for every `S2`.
    fn restate_impl(&self) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let target_state = &self.target_state.ident;
        let target_ty = &self.target_ty;
        let (_, ty_generics, _) =
            self.item_struct.generics.split_for_impl();
        let generics = self.target_generics();
        let (impl_generics, _, where_clause) = generics.split_for_impl();

        quote! {
            impl #impl_generics ::groupoid::Restate<#target_state>
                for #struct_ident #ty_generics #where_clause
            {
                type Target = #target_ty;
            }
        }
    }

    /// The struct's generics plus the target state.
    fn target_generics(&self) -> Generics {
        let mut generics = self.item_struct.generics.clone();
        generics
            .params
            .push(GenericParam::Type(self.target_state.clone()));
        generics
    }

    /// `TransmutableState<S2>` for `Struct<S>`, for every `S2` with a
    /// matching layout.
    fn transmutable_state_impl(&self, align: &Alignment) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let target_state = &self.target_state.ident;
        let (_, ty_generics, _) =
            self.item_struct.generics.split_for_impl();

        let mut generics = self.target_generics();
        generics.make_where_clause().predicates.extend(
            self.projections.iter().map(|projection| {
                self.layout_predicate(projection, align)
            }),
        );
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let layout_check = self.layout_check();

        quote! {
            // SAFETY: the where-clause gives every projection one size
            // in both states, and the `repr` fixes the field order.
            // `LAYOUT_CHECK` rejects any alignment or field offset that
            // `align = N` lets differ.
            unsafe impl #impl_generics ::groupoid::TransmutableState<#target_state>
                for #struct_ident #ty_generics #where_clause
            {
                #layout_check
            }
        }
    }

    /// `TransmutableState::LAYOUT_CHECK`, extended with one offset
    /// assertion per field.
    ///
    /// `align = N` pins the container's alignment but not its fields', so
    /// a field after a projection can sit at another offset in the
    /// target state.
    fn layout_check(&self) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let target_ty = &self.target_ty;
        let align_msg = format!(
            "raise `align = N` on `{struct_ident}` to at least the \
             largest alignment among its states"
        );
        let offset_asserts = self.item_struct.fields.members().map(|member| {
            let msg = format!(
                "move field `{}` to the start of `{struct_ident}`, or \
                 transmute only between states whose types share an \
                 alignment: its offset differs in the target state",
                quote!(#member),
            );
            quote! {
                ::core::assert!(
                    ::core::mem::offset_of!(Self, #member)
                        == ::core::mem::offset_of!(#target_ty, #member),
                    #msg
                );
            }
        });

        quote! {
            const LAYOUT_CHECK: () = {
                ::core::assert!(
                    ::core::mem::align_of::<Self>()
                        == ::core::mem::align_of::<#target_ty>(),
                    #align_msg
                );
                #(#offset_asserts)*
                ::core::assert!(
                    ::core::mem::size_of::<Self>()
                        == ::core::mem::size_of::<#target_ty>(),
                    "`Self` and `Target` must have the same size"
                );
            };
        }
    }

    /// `S2::__GroupoidLayoutP: SameLayout<S::__GroupoidLayoutP>`, or only
    /// `SameSize` under `align = N`.
    fn layout_predicate(
        &self,
        projection: &Ident,
        align: &Alignment,
    ) -> WherePredicate {
        let state = &self.state;
        let target_state = &self.target_state.ident;
        let layout = crate::naming::layout_assoc_ident(projection);

        match align {
            Alignment::Inferred => parse_quote! {
                #target_state::#layout: ::groupoid::SameLayout<#state::#layout>
            },
            Alignment::Forced(_) => parse_quote! {
                <#target_state::#layout as ::groupoid::TypeLayout>::Size:
                    ::groupoid::SameSize<<#state::#layout as ::groupoid::TypeLayout>::Size>
            },
        }
    }
}

/// `#[typestate(state = S, unsafe_transmute = true, align = N)]`
pub(crate) struct TypeStateArgs {
    state: Option<Ident>,
    transmute: Transmute,
}

impl Parse for TypeStateArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut state: Option<Ident> = None;
        let mut unsafe_transmute: Option<LitBool> = None;
        let mut align: Option<LitInt> = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            match key.to_string().as_str() {
                "state" => state.parse_once(&key, input)?,
                "unsafe_transmute" => {
                    unsafe_transmute.parse_once(&key, input)?
                }
                "align" => align.parse_once(&key, input)?,
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        "expected `state = <Ident>`, `unsafe_transmute = \
                         <bool>` or `align = <integer literal>`",
                    ));
                }
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        let align = align.map_or(Alignment::Inferred, Alignment::Forced);
        let transmute =
            if unsafe_transmute.is_some_and(|flag| flag.value()) {
                Transmute::On(align)
            } else if let Alignment::Forced(n) = align {
                return Err(syn::Error::new(
                    n.span(),
                    "add `unsafe_transmute = true` to use `align`",
                ));
            } else {
                Transmute::Off
            };

        Ok(TypeStateArgs { state, transmute })
    }
}

/// Whether `#[typestate]` implements `TransmutableState`.
pub(crate) enum Transmute {
    Off,
    On(Alignment),
}

/// The container's alignment.
pub(crate) enum Alignment {
    Inferred,
    /// `align = N`
    Forced(LitInt),
}

#[ext]
impl ItemStruct {
    /// The type parameter named `state`, or the only one.
    fn state_param(
        &mut self,
        state: Option<&Ident>,
    ) -> syn::Result<&mut TypeParam> {
        match state {
            Some(state) => {
                let msg = format!(
                    "set `state` to one of the generic type parameters \
                     of `{}`",
                    self.ident
                );
                self.generics
                    .type_param_mut(state)
                    .ok_or_else(|| syn::Error::new(state.span(), msg))
            }
            None => self.infer_state_param(),
        }
    }

    /// The struct's only generic type parameter.
    fn infer_state_param(&mut self) -> syn::Result<&mut TypeParam> {
        let mut type_params = self.generics.type_params_mut();

        type_params
            .next()
            .filter(|_| type_params.next().is_none())
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    &self.ident,
                    "add `state = <Ident>` to `#[typestate]` to name the \
                     state parameter; only a struct with one generic \
                     type parameter can leave it out",
                )
            })
    }

    /// The distinct associated types the fields project through the
    /// state (`Value` in `S::Value`), at any depth, in order of first
    /// appearance.
    fn projections(&self, state: &Ident) -> syn::Result<Vec<Ident>> {
        let mut projections: Vec<Ident> = Vec::new();

        for projection in self
            .fields
            .iter()
            .flat_map(|field| field.ty.projections(state))
        {
            if projection == "Marker" {
                return Err(syn::Error::new(
                    projection.span(),
                    format!(
                        "replace `{state}::Marker` with \
                         `PhantomData<{state}>`: a group marker has no \
                         value to convert"
                    ),
                ));
            }
            if !projections.contains(&projection) {
                projections.push(projection);
            }
        }

        Ok(projections)
    }

    /// Checks that every field keeps its layout across states.
    fn require_transmutable_layout(
        &self,
        state: &Ident,
    ) -> syn::Result<()> {
        for field in self.fields.iter() {
            if !field.is_transmutable(state) {
                return Err(syn::Error::new_spanned(
                    &field.ty,
                    format!(
                        "make this field `{state}::Assoc`, a ZST or a \
                         type without `{state}`, or remove \
                         `unsafe_transmute = true` and convert with \
                         `morph`"
                    ),
                ));
            }
        }

        Ok(())
    }

    /// Adds `#[repr(C)]` if missing, and `#[repr(align(N))]` when forced.
    fn ensure_repr(&mut self, align: &Alignment) -> syn::Result<()> {
        let reprs: Vec<&Attribute> = self
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("repr"))
            .collect();

        match reprs.as_slice() {
            [] => self.attrs.push(parse_quote!(#[repr(C)])),
            [first, ..]
                if !reprs.iter().any(|attr| attr.guarantees_layout()) =>
            {
                return Err(syn::Error::new_spanned(
                    first,
                    "add `C` to this `#[repr(..)]`: `unsafe_transmute = \
                     true` needs a guaranteed field layout",
                ));
            }
            _ => {}
        }

        if let Alignment::Forced(n) = align {
            self.attrs.push(parse_quote!(#[repr(align(#n))]));
        }

        Ok(())
    }

    /// `Wrap<S> -> Wrap<target_state>`
    fn with_state(
        &self,
        state: &Ident,
        target_state: &Ident,
    ) -> TokenStream {
        let struct_ident = &self.ident;
        let args = self.generics.params.iter().map(|param| match param {
            GenericParam::Type(param) if param.ident == *state => {
                quote!(#target_state)
            }
            GenericParam::Type(TypeParam { ident, .. })
            | GenericParam::Const(ConstParam { ident, .. }) => {
                quote!(#ident)
            }
            GenericParam::Lifetime(LifetimeParam { lifetime, .. }) => {
                quote!(#lifetime)
            }
        });

        quote!(#struct_ident<#(#args),*>)
    }
}

#[ext]
impl Field {
    /// Whether this field is `S::Value`, a ZST, or state-independent.
    fn is_transmutable(&self, state: &Ident) -> bool {
        self.ty.state_projection(state).is_some()
            || self.ty.is_zst()
            || !self.ty.mentions_ident(state)
    }
}

#[ext]
impl Type {
    /// `Option<S::Value> -> [Value]`
    fn projections(&self, state: &Ident) -> Vec<Ident> {
        let mut projections = Projections {
            state,
            found: Vec::new(),
        };
        projections.visit_type(self);
        projections.found
    }

    /// `Assoc` when this type is `S::Assoc, or (S::Assoc)`.
    fn state_projection(&self, state: &Ident) -> Option<&Ident> {
        let Type::Path(TypePath {
            qself: None,
            path:
                Path {
                    leading_colon: None,
                    segments,
                },
            ..
        }) = self.peeled()
        else {
            return None;
        };

        match segments.iter().collect::<Vec<_>>().as_slice() {
            [state_seg, type_seg] => (state_seg.ident == *state
                && state_seg.arguments == PathArguments::None
                && type_seg.arguments == PathArguments::None)
                .then_some(&type_seg.ident),
            _ => None,
        }
    }
}

/// Collects the `Assoc` of every `S::Assoc` it visits.
struct Projections<'a> {
    state: &'a Ident,
    found: Vec<Ident>,
}

impl<'ast> Visit<'ast> for Projections<'_> {
    fn visit_type(&mut self, ty: &'ast Type) {
        match ty.state_projection(self.state) {
            Some(assoc) => self.found.push(assoc.clone()),
            None => visit::visit_type(self, ty),
        }
    }
}
