use extend::ext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, ConstParam, Field, Fields, GenericArgument, GenericParam,
    Generics, Ident, Index, ItemStruct, LifetimeParam, LitBool, LitInt,
    Path, PathArguments, Token, Type, TypeParam, TypePath, TypeTuple,
    WherePredicate,
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
        if !projections.is_empty() {
            item_struct.require_morphable_layers(&state, &projections)?;
        }

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
        let morph_impl = self.morph_impl();

        quote! {
            #item_struct

            #with_state_impl

            #transmutable_state_impl

            #morph_impl
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

    /// `TransmutableState<S2>` for `Struct<S>`, for every `S2` with a
    /// matching layout.
    fn transmutable_state_impl(&self, align: &Alignment) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let target_state = &self.target_state.ident;
        let target_ty = &self.target_ty;
        let (_, ty_generics, _) =
            self.item_struct.generics.split_for_impl();

        let mut generics = self.item_struct.generics.clone();
        generics
            .params
            .push(GenericParam::Type(self.target_state.clone()));
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
                type Target = #target_ty;

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

    /// The inherent `morph_with`, which rebuilds the struct for another
    /// state by value, and the `{Struct}Morph` it takes when the fields
    /// use several projections.
    fn morph_impl(&self) -> Option<TokenStream> {
        let (morph_with, morph_struct) = match self.projections.as_slice()
        {
            [] => return None,
            [projection] => (self.closure_morph_with(projection), None),
            _ => (self.fields_morph_with(), Some(self.morph_struct())),
        };
        let struct_ident = &self.item_struct.ident;
        let (impl_generics, ty_generics, where_clause) =
            self.item_struct.generics.split_for_impl();

        Some(quote! {
            #morph_struct

            impl #impl_generics #struct_ident #ty_generics #where_clause {
                #morph_with
            }
        })
    }

    /// `morph_with(f)`, converting the one projection with the closure
    /// `f`.
    fn closure_morph_with(&self, projection: &Ident) -> TokenStream {
        let state = &self.state;
        let target_state = &self.target_state.ident;
        let target_ty = &self.target_ty;
        let f = crate::naming::morph_fn_ident();
        let (generics, body) = self.morph_parts(&Leaves::Closure(&f));
        let (impl_generics, _, where_clause) = generics.split_for_impl();

        quote! {
            /// Rebuilds `self` for the target state, converting every
            /// projection through the state with `f`.
            ///
            /// Tuples are rebuilt in place, and every other wrapper goes
            /// through `groupoid::Morph`, one layer at a time. Fields
            /// that don't mention the state move unchanged.
            pub fn morph_with #impl_generics (
                self,
                mut #f: impl FnMut(#state::#projection)
                    -> #target_state::#projection,
            ) -> #target_ty #where_clause {
                #body
            }
        }
    }

    /// `morph_with({Struct}Morph { .. })`, converting each projection
    /// with its own closure.
    fn fields_morph_with(&self) -> TokenStream {
        let state = &self.state;
        let target_state = &self.target_state.ident;
        let target_ty = &self.target_ty;
        let morph_struct =
            crate::naming::struct_morph_ident(&self.item_struct.ident);
        let f = crate::naming::morph_fn_ident();
        let (generics, body) = self.morph_parts(&Leaves::Fields);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let bindings = self.projections.iter().map(|projection| {
            let field = crate::naming::morph_method_ident(projection);
            let binding = crate::naming::morph_binding_ident(projection);
            quote!(#field: #binding)
        });
        let doc = format!(
            "Rebuilds `self` for the target state, converting each \
             projection through the state with its closure in \
             [`{morph_struct}`]."
        );

        quote! {
            #[doc = #doc]
            ///
            /// Tuples are rebuilt in place, and every other wrapper goes
            /// through `groupoid::Morph`, one layer at a time. Fields
            /// that don't mention the state move unchanged.
            pub fn morph_with #impl_generics (
                self,
                #f: #morph_struct<'_, #state, #target_state>,
            ) -> #target_ty #where_clause {
                let #morph_struct { #(#bindings),* } = #f;
                #body
            }
        }
    }

    /// `{Struct}Morph`, one `&mut dyn FnMut` per projection.
    ///
    /// The states are parameters of the struct, so each closure infers
    /// its argument type from its field. A generic `F: FnMut(..)` field
    /// would get a fresh type variable in the struct literal instead.
    fn morph_struct(&self) -> TokenStream {
        let vis = &self.item_struct.vis;
        let struct_ident = &self.item_struct.ident;
        let morph_struct = crate::naming::struct_morph_ident(struct_ident);
        let lifetime = crate::naming::morph_lifetime();
        let state = &self.state;
        let target_state = &self.target_state.ident;
        let state_param = TypeParam {
            ident: state.clone(),
            ..self.target_state.clone()
        };
        let target_param = &self.target_state;
        let fields = self.projections.iter().map(|projection| {
            let field = crate::naming::morph_method_ident(projection);
            let doc = format!(
                "Converts `{state}::{projection}` into the target \
                 state's."
            );
            quote! {
                #[doc = #doc]
                pub #field: &#lifetime mut dyn FnMut(#state::#projection)
                    -> #target_state::#projection
            }
        });
        let doc = format!(
            "The conversions [`{struct_ident}::morph_with`] applies, one \
             per associated type of `{state}` that `{struct_ident}` uses."
        );

        quote! {
            #[doc = #doc]
            #vis struct #morph_struct<#lifetime, #state_param, #target_param> {
                #(#fields,)*
            }
        }
    }

    /// The struct literal rebuilding `self` for the target state through
    /// `leaves`, and the method's generics: the target state, bounded by
    /// the `Morph` calls the literal makes.
    fn morph_parts(&self, leaves: &Leaves) -> (Generics, TokenStream) {
        let elem = crate::naming::morph_elem_ident();
        let ctx = MorphCtx {
            state: &self.state,
            target_state: &self.target_state.ident,
            projections: &self.projections,
            leaves,
            elem: &elem,
        };
        let mut predicates = Vec::new();
        let body = self.morph_body(&ctx, &mut predicates);

        let mut generics = Generics::default();
        generics
            .params
            .push(GenericParam::Type(self.target_state.clone()));
        generics.make_where_clause().predicates.extend(predicates);

        (generics, body)
    }

    /// The struct literal rebuilding `self`, pushing its `Morph` bounds
    /// onto `predicates`.
    fn morph_body(
        &self,
        ctx: &MorphCtx,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        let struct_ident = &self.item_struct.ident;

        match &self.item_struct.fields {
            Fields::Named(named) => {
                let fields = named.named.iter().map(|field| {
                    let ident = field
                        .ident
                        .as_ref()
                        .expect("named fields have identifiers");
                    let expr = field.ty.morph_expr(
                        ctx,
                        quote!(self.#ident),
                        predicates,
                    );
                    quote!(#ident: #expr)
                });
                quote!(#struct_ident { #(#fields),* })
            }
            Fields::Unnamed(unnamed) => {
                let fields = unnamed.unnamed.iter().enumerate().map(
                    |(i, field)| {
                        let index = Index::from(i);
                        field.ty.morph_expr(
                            ctx,
                            quote!(self.#index),
                            predicates,
                        )
                    },
                );
                quote!(#struct_ident(#(#fields),*))
            }
            Fields::Unit => unreachable!(
                "a unit struct has no field to project through the state"
            ),
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

    /// Checks that every layer `morph_with` converts with one `Morph`
    /// call has one generic argument and one projection to convert.
    fn require_morphable_layers(
        &self,
        state: &Ident,
        projections: &[Ident],
    ) -> syn::Result<()> {
        let layers = self
            .fields
            .iter()
            .flat_map(|field| field.ty.opaque_layers(state));

        for layer in layers {
            if layer.state_args(state).len() > 1 {
                return Err(syn::Error::new_spanned(
                    layer,
                    format!(
                        "give this wrapper one generic argument that \
                         mentions `{state}`, such as a local \
                         `Wrapper<{state}::Assoc>` that implements \
                         `groupoid::Morph`"
                    ),
                ));
            }
            if layer.opaque_projection(state, projections).is_none() {
                return Err(syn::Error::new_spanned(
                    layer,
                    format!(
                        "split this type into one part per associated \
                         type of `{state}`, such as a tuple \
                         `(Wrapper<{state}::A>, Wrapper<{state}::B>)`: \
                         `morph_with` converts it through one projection"
                    ),
                ));
            }
        }

        Ok(())
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
                         `morph_with`"
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

    /// The expression rebuilding `expr` for the target state, pushing its
    /// `Morph` bounds onto `predicates`.
    fn morph_expr(
        &self,
        ctx: &MorphCtx,
        expr: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        match self.morph_shape(ctx.state) {
            MorphShape::Unchanged => expr,
            MorphShape::Direct(projection) => {
                ctx.leaves.call(projection, expr)
            }
            MorphShape::Zst(value) => value,
            MorphShape::Tuple(tuple) => {
                tuple.morph_tuple(ctx, expr, predicates)
            }
            MorphShape::Wrapped(inner) => {
                let MorphCtx {
                    state,
                    target_state,
                    elem,
                    ..
                } = ctx;
                let inner_expr =
                    inner.morph_expr(ctx, quote!(#elem), predicates);
                self.morph_call(
                    expr,
                    quote!(#inner),
                    inner.with_ident_renamed(state, target_state),
                    quote!(|#elem| #inner_expr),
                    ctx,
                    predicates,
                )
            }
            MorphShape::Opaque => {
                let MorphCtx {
                    state,
                    target_state,
                    projections,
                    leaves,
                    elem,
                } = ctx;
                let projection = self
                    .opaque_projection(state, projections)
                    .expect("`new` rejects layers without one projection");
                let leaf = leaves.call(projection, quote!(#elem));
                self.morph_call(
                    expr,
                    quote!(#state::#projection),
                    quote!(#target_state::#projection),
                    quote!(|#elem| #leaf),
                    ctx,
                    predicates,
                )
            }
        }
    }

    /// How `morph_expr` rebuilds a value of this type.
    fn morph_shape(&self, state: &Ident) -> MorphShape<'_> {
        if !self.mentions_ident(state) {
            return MorphShape::Unchanged;
        }
        if let Some(projection) = self.state_projection(state) {
            return MorphShape::Direct(projection);
        }
        if let Some(value) = self.zst_value() {
            return MorphShape::Zst(value);
        }
        if let Type::Tuple(tuple) = self.peeled() {
            return MorphShape::Tuple(tuple);
        }

        self.morph_inner(state)
            .map_or(MorphShape::Opaque, MorphShape::Wrapped)
    }

    /// `Option<[S::Value; 2]> -> [S::Value; 2]`
    fn morph_inner(&self, state: &Ident) -> Option<&Type> {
        if let Type::Array(array) = self.peeled() {
            return Some(&array.elem);
        }

        match self.state_args(state).as_slice() {
            [inner] => Some(inner),
            _ => None,
        }
    }

    /// `Result<S::Value, [S::Value; 2]> -> [S::Value, [S::Value; 2]]`
    fn state_args(&self, state: &Ident) -> Vec<&Type> {
        let Type::Path(TypePath {
            qself: None, path, ..
        }) = self.peeled()
        else {
            return Vec::new();
        };
        let Some(PathArguments::AngleBracketed(args)) =
            path.segments.last().map(|segment| &segment.arguments)
        else {
            return Vec::new();
        };

        let mut found = Vec::new();
        for arg in &args.args {
            let GenericArgument::Type(ty) = arg else {
                continue;
            };
            if ty.mentions_ident(state) && !found.contains(&ty) {
                found.push(ty);
            }
        }
        found
    }

    /// The layers `morph_with` converts with one `Morph` call.
    ///
    /// `(S::A, Option<Result<S::A, S::B>>) -> [Result<S::A, S::B>]`
    fn opaque_layers(&self, state: &Ident) -> Vec<&Type> {
        match self.morph_shape(state) {
            MorphShape::Unchanged
            | MorphShape::Direct(_)
            | MorphShape::Zst(_) => Vec::new(),
            MorphShape::Tuple(tuple) => tuple
                .elems
                .iter()
                .flat_map(|ty| ty.opaque_layers(state))
                .collect(),
            MorphShape::Wrapped(inner) => inner.opaque_layers(state),
            MorphShape::Opaque => vec![self],
        }
    }

    /// The projection an opaque layer converts through: the one it
    /// mentions, or the struct's only one when it mentions none.
    fn opaque_projection<'p>(
        &self,
        state: &Ident,
        projections: &'p [Ident],
    ) -> Option<&'p Ident> {
        let mentioned = self.projections(state);
        let found: Vec<&Ident> = projections
            .iter()
            .filter(|projection| mentioned.contains(projection))
            .collect();

        match (found.as_slice(), projections) {
            ([projection], _) => Some(projection),
            ([], [projection]) => Some(projection),
            _ => None,
        }
    }

    /// A `Morph` call on `expr` and the bound it needs.
    fn morph_call(
        &self,
        expr: TokenStream,
        src: TokenStream,
        dst: TokenStream,
        convert: TokenStream,
        ctx: &MorphCtx,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        let target_ty =
            self.with_ident_renamed(ctx.state, ctx.target_state);

        predicates.push(parse_quote!(
            #self: ::groupoid::Morph<#src, #dst, Output = #target_ty>
        ));
        quote!(<#self as ::groupoid::Morph<#src, #dst>>::morph(#expr, &mut #convert))
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

#[ext]
impl TypeTuple {
    /// `(T, ..)` rebuilt element by element.
    fn morph_tuple(
        &self,
        ctx: &MorphCtx,
        expr: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        let bindings: Vec<Ident> = (0..self.elems.len())
            .map(crate::naming::tuple_binding_ident)
            .collect();
        let elems =
            self.elems.iter().zip(&bindings).map(|(ty, binding)| {
                ty.morph_expr(ctx, quote!(#binding), predicates)
            });
        quote!({
            let (#(#bindings,)*) = #expr;
            (#(#elems,)*)
        })
    }
}

/// How `Type::morph_expr` rebuilds a value for the target state.
enum MorphShape<'a> {
    /// A type that doesn't mention the state, moved as is.
    Unchanged,
    /// `S::Value`, converted by its leaf.
    Direct(&'a Ident),
    /// A ZST, created fresh.
    Zst(TokenStream),
    /// `(T, ..)`
    Tuple(&'a TypeTuple),
    /// `Wrapper<T>` or `[T; N]`, one `Morph` layer around `T`.
    Wrapped(&'a Type),
    /// Anything else, one `Morph` call on the projection.
    Opaque,
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

/// What `Type::morph_expr` needs to convert a projection.
struct MorphCtx<'a> {
    state: &'a Ident,
    target_state: &'a Ident,
    /// Every projection the struct uses.
    projections: &'a [Ident],
    leaves: &'a Leaves<'a>,
    /// The closure binding for one element of a mapped or boxed value.
    elem: &'a Ident,
}

/// How the generated method converts one projected value.
enum Leaves<'a> {
    /// `morph_with(f)`, one closure for the only projection: `f(v)`.
    Closure(&'a Ident),
    /// `morph_with({Struct}Morph { .. })`, one closure per projection,
    /// destructured into bindings: `__groupoid_value(v)`.
    Fields,
}

impl Leaves<'_> {
    /// The expression converting `arg`, a value of `S::projection`.
    fn call(&self, projection: &Ident, arg: TokenStream) -> TokenStream {
        match self {
            Leaves::Closure(f) => quote!(#f(#arg)),
            Leaves::Fields => {
                let binding =
                    crate::naming::morph_binding_ident(projection);
                quote!(#binding(#arg))
            }
        }
    }
}
