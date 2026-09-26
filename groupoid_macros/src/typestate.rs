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
    /// The original struct with the state parameter's bounds extended
    /// with `::groupoid::State`, and its `repr` settled when transmute is
    /// on.
    item_struct: ItemStruct,
    /// The generic type parameter carrying the state.
    state: Ident,
    /// The state being moved to, with the state parameter's bounds.
    target_state: TypeParam,
    /// The struct's type in the target state.
    ///
    /// `Wrap<S> -> Wrap<__GroupoidTargetState>`
    target_ty: TokenStream,
    /// Whether `SizedWithState` and `TransmutableState` are derived too.
    transmute: Transmute,
    /// The one associated type the fields project through the state
    /// (`Value` in `S::Value`), if any.
    projection: Option<Ident>,
}

impl TypeState {
    /// Resolves the state parameter, bounds it by `::groupoid::State`,
    /// checks that `morph_with` can peel every field, and checks the
    /// struct's layout when transmute is on.
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
        let projection = item_struct.unique_projection(&state)?;
        if projection.is_some() {
            item_struct.require_single_state_args(&state)?;
        }

        if let Transmute::On(align) = &args.transmute {
            item_struct.require_transmutable_layout(&state)?;
            if projection.is_none() {
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
            projection,
        })
    }

    /// The struct followed by every impl `#[typestate]` derives for it.
    pub(crate) fn generate_typestate_impls(&self) -> TokenStream {
        let item_struct = &self.item_struct;
        let with_state_impl = self.with_state_impl();
        let transmute_impls = match &self.transmute {
            Transmute::On(align) => Some(self.transmute_impls(align)),
            Transmute::Off => None,
        };
        let morph_impl = self
            .projection
            .as_ref()
            .map(|projection| self.morph_impl(projection));

        quote! {
            #item_struct

            #with_state_impl

            #transmute_impls

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

    /// `SizedWithState<N, A>` and `TransmutableState<S2, N, A>`, with `A`
    /// either the state's own alignment or the one forced by `align = A`.
    fn transmute_impls(&self, align: &Alignment) -> TokenStream {
        let sized_with_state_impl = self.sized_with_state_impl(align);
        let transmutable_state_impl = self.transmutable_state_impl(align);

        quote! {
            #sized_with_state_impl

            #transmutable_state_impl
        }
    }

    /// `SizedWithState<N, A>` for the struct, pinned by the state's
    /// `#[size(N)]` template type.
    fn sized_with_state_impl(&self, align: &Alignment) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let (_, ty_generics, _) =
            self.item_struct.generics.split_for_impl();

        let mut generics = self.item_struct.generics.clone();
        let align_arg =
            generics.pin_states_to_shared_layout(&[&self.state], align);
        let (impl_generics, _, where_clause) = generics.split_for_impl();

        quote! {
            impl #impl_generics ::groupoid::SizedWithState<__GROUPOID_SIZE, #align_arg>
                for #struct_ident #ty_generics #where_clause {}
        }
    }

    /// `TransmutableState<S2, N, A>` for `Struct<S>` with
    /// `Target = Struct<S2>`, generic over every `S2` with `S`'s bounds
    /// and layout.
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
        let align_arg = generics.pin_states_to_shared_layout(
            &[&self.state, target_state],
            align,
        );
        let (impl_generics, _, where_clause) = generics.split_for_impl();

        quote! {
            // SAFETY: `Self` and `Target` differ only in the state, every
            // field is a projection, a ZST or state-independent, and both
            // projections share size and alignment, so `repr(C)` lays
            // them out identically.
            unsafe impl #impl_generics ::groupoid::TransmutableState<
                #target_state, __GROUPOID_SIZE, #align_arg
            > for #struct_ident #ty_generics #where_clause {
                type Target = #target_ty;
            }
        }
    }

    /// The inherent `morph_with` method, which rebuilds the struct for
    /// another state by value.
    fn morph_impl(&self, projection: &Ident) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let state = &self.state;
        let target_state = &self.target_state.ident;
        let target_ty = &self.target_ty;
        let f = &crate::naming::morph_fn_ident();
        let elem = &crate::naming::morph_elem_ident();
        let (impl_generics, ty_generics, where_clause) =
            self.item_struct.generics.split_for_impl();

        let ctx = MorphCtx {
            state,
            target_state,
            projection,
            f,
            elem,
        };
        let mut predicates = Vec::new();
        let body = self.morph_body(&ctx, &mut predicates);

        let mut generics = Generics::default();
        generics
            .params
            .push(GenericParam::Type(self.target_state.clone()));
        generics.make_where_clause().predicates.extend(predicates);
        let (with_impl_generics, _, with_where_clause) =
            generics.split_for_impl();

        quote! {
            impl #impl_generics #struct_ident #ty_generics #where_clause {
                /// Rebuilds `self` for the target state, converting every
                /// projection through the state with `f`.
                ///
                /// Tuples are rebuilt in place, and every other wrapper goes
                /// through `groupoid::Morph`, one layer at a time. Fields
                /// that don't mention the state move unchanged.
                pub fn morph_with #with_impl_generics (
                    self,
                    mut #f: impl FnMut(#state::#projection)
                        -> #target_state::#projection,
                ) -> #target_ty #with_where_clause {
                    #body
                }
            }
        }
    }

    /// The struct literal rebuilding `self` field by field for the target
    /// state, pushing the `Morph` bounds it needs onto `predicates`.
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

/// `#[typestate]`'s arguments: an optional `state = <Ident>`, an optional
/// `unsafe_transmute = <bool>` and an optional `align = <integer
/// literal>`, in any order, each at most once.
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

/// Whether `#[typestate]` also derives the in-place `transmute_state`.
pub(crate) enum Transmute {
    /// Default, only `WithState` and `morph_with`.
    Off,
    /// `unsafe_transmute = true`, additionally implements `SizedWithState`
    /// and `TransmutableState`.
    On(Alignment),
}

/// How the container's alignment is pinned.
pub(crate) enum Alignment {
    /// No `align` argument.
    Inferred,
    /// `align = N`. The container is forced to `#[repr(align(N))]`.
    Forced(LitInt),
}

#[ext]
impl ItemStruct {
    /// The generic type parameter named `state`, or the only one when no
    /// name is given.
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

    /// The one associated type the fields project through the state
    /// (`Value` in `S::Value`), at any depth.
    fn unique_projection(
        &self,
        state: &Ident,
    ) -> syn::Result<Option<Ident>> {
        let mut projections = self
            .fields
            .iter()
            .flat_map(|field| field.ty.projections(state));

        let Some(first) = projections.next() else {
            return Ok(None);
        };
        if let Some(found) = projections.find(|found| *found != first) {
            return Err(syn::Error::new(
                found.span(),
                format!(
                    "project every field through the same associated \
                     type of `{state}`, not both `{state}::{first}` and \
                     `{state}::{found}`"
                ),
            ));
        }

        Ok(Some(first))
    }

    /// Checks that every wrapper `morph_with` peels has one generic
    /// argument to morph.
    fn require_single_state_args(&self, state: &Ident) -> syn::Result<()> {
        for field in self.fields.iter() {
            if let Some(layer) = field.ty.ambiguous_layer(state) {
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
        }

        Ok(())
    }

    /// Checks that every field's layout can be pinned across states.
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

    /// Guarantees the struct's layout with `#[repr(C | transparent)]`, and
    /// adds `#[repr(align(N))]` for a forced alignment.
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

    /// This struct's type with the `state` parameter swapped for
    /// `target_state`.
    ///
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
impl Generics {
    /// Requires all `states` to share one size via `SizedGroup`, and one
    /// alignment via `AlignedGroup` unless it is forced. Returns the
    /// alignment to use.
    fn pin_states_to_shared_layout(
        &mut self,
        states: &[&Ident],
        align: &Alignment,
    ) -> TokenStream {
        self.params.push(parse_quote!(const __GROUPOID_SIZE: usize));
        self.make_where_clause()
            .predicates
            .extend(states.iter().map(|state| -> WherePredicate {
                parse_quote!(#state::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>)
            }));

        match align {
            Alignment::Forced(n) => quote!(#n),
            Alignment::Inferred => {
                self.params
                    .push(parse_quote!(const __GROUPOID_ALIGN: usize));
                self.make_where_clause()
                    .predicates
                    .extend(states.iter().map(|state| -> WherePredicate {
                        parse_quote!(#state::Marker: ::groupoid::AlignedGroup<__GROUPOID_ALIGN>)
                    }));
                quote!(__GROUPOID_ALIGN)
            }
        }
    }
}

#[ext]
impl Field {
    /// Whether this field is a bare `S::Value`, a ZST, or doesn't mention
    /// the state. `Option<S::Value>` is not.
    fn is_transmutable(&self, state: &Ident) -> bool {
        self.ty.state_projection(state).is_some()
            || self.ty.is_zst()
            || !self.ty.mentions_ident(state)
    }
}

#[ext]
impl Type {
    /// The `Assoc` of every `S::Assoc` inside this type.
    ///
    /// `Option<S::Value> -> [Value]`
    fn projections(&self, state: &Ident) -> Vec<Ident> {
        let mut projections = Projections {
            state,
            found: Vec::new(),
        };
        projections.visit_type(self);
        projections.found
    }

    /// The expression rebuilding `expr`, a value of this type, for the
    /// target state. Pushes the `Morph` bounds it needs onto
    /// `predicates`.
    fn morph_expr(
        &self,
        ctx: &MorphCtx,
        expr: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        match self.morph_shape(ctx.state) {
            MorphShape::Unchanged => expr,
            MorphShape::Direct => {
                let f = ctx.f;
                quote!(#f(#expr))
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
                    projection,
                    f,
                    ..
                } = ctx;
                self.morph_call(
                    expr,
                    quote!(#state::#projection),
                    quote!(#target_state::#projection),
                    quote!(#f),
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
        if self.state_projection(state).is_some() {
            return MorphShape::Direct;
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

    /// The one generic argument that mentions the state, or the element
    /// of an array.
    ///
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

    /// The distinct generic arguments of this path type that mention the
    /// state.
    ///
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

    /// The first layer `morph_with` would reach that has several
    /// different generic arguments mentioning the state.
    ///
    /// `Option<Result<S::Value, [S::Value; 2]>> -> Result<..>`
    fn ambiguous_layer(&self, state: &Ident) -> Option<&Type> {
        match self.morph_shape(state) {
            MorphShape::Unchanged
            | MorphShape::Direct
            | MorphShape::Zst(_) => None,
            MorphShape::Tuple(tuple) => {
                tuple.elems.iter().find_map(|ty| ty.ambiguous_layer(state))
            }
            MorphShape::Wrapped(inner) => inner.ambiguous_layer(state),
            MorphShape::Opaque => {
                (self.state_args(state).len() > 1).then_some(self)
            }
        }
    }

    /// A `Morph<src, dst>` call on `expr` with the closure `convert`,
    /// and the bound it needs.
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

    /// `Value` when this type is `S::Value`.
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
    /// `(T, ..)`, destructured and rebuilt element by element.
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
    /// `S::Value`, passed to `f`.
    Direct,
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
    /// `Value` in `S::Value`.
    projection: &'a Ident,
    /// The conversion closure.
    f: &'a Ident,
    /// The closure binding for one element of a mapped or boxed value.
    elem: &'a Ident,
}
