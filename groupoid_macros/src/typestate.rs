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
    /// Resolves the state parameter, bounds it by `::groupoid::State`, and
    /// checks the struct's layout when transmute is on.
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

        if let Transmute::On(align) = &args.transmute {
            item_struct.require_transmutable_layout(&state)?;
            if projection.is_none() {
                return Err(syn::Error::new_spanned(
                    &item_struct.ident,
                    format!(
                        "`unsafe_transmute = true` needs at least one \
                         field to be a bare `{state}::Assoc` projection",
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
    ///
    /// ```text
    /// #[typestate(unsafe_transmute = true)]
    /// struct Wrap<S> { value: S::Value }
    /// ```
    /// expands to
    /// ```text
    /// #[repr(C)]
    /// struct Wrap<S: ::groupoid::State> { value: S::Value }
    /// impl<S: ::groupoid::State> ::groupoid::WithState for Wrap<S> { type State = S; }
    /// impl<..> ::groupoid::SizedWithState<__GROUPOID_SIZE, __GROUPOID_ALIGN> for Wrap<S> where .. {}
    /// unsafe impl<..> ::groupoid::TransmutableState<__GroupoidTargetState, ..> for Wrap<S> where .. {
    ///     type Target = Wrap<__GroupoidTargetState>;
    /// }
    /// impl<S: ::groupoid::State> Wrap<S> { pub fn restate_with<..>(self, ..) -> .. { .. } }
    /// ```
    pub(crate) fn generate_typestate_impls(&self) -> TokenStream {
        let item_struct = &self.item_struct;
        let with_state_impl = self.with_state_impl();
        let transmute_impls = match &self.transmute {
            Transmute::On(align) => Some(self.transmute_impls(align)),
            Transmute::Off => None,
        };
        let restate_impl = self
            .projection
            .as_ref()
            .map(|projection| self.restate_impl(projection));

        quote! {
            #item_struct

            #with_state_impl

            #transmute_impls

            #restate_impl
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
    /// `#[size(N)]` blueprint type.
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

    /// The inherent `restate_with` method, which rebuilds the struct for
    /// another state by value.
    fn restate_impl(&self, projection: &Ident) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let state = &self.state;
        let target_state = &self.target_state.ident;
        let target_ty = &self.target_ty;
        let f = &crate::naming::restate_fn_ident();
        let elem = &crate::naming::restate_elem_ident();
        let (impl_generics, ty_generics, where_clause) =
            self.item_struct.generics.split_for_impl();

        let ctx = RestateCtx {
            state,
            target_state,
            projection,
            f,
            elem,
        };
        let mut predicates = Vec::new();
        let body = self.restate_body(&ctx, &mut predicates);

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
                /// through `groupoid::Restate`, one layer at a time. Fields
                /// that don't mention the state move unchanged.
                pub fn restate_with #with_impl_generics (
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
    /// state, pushing the `Restate` bounds it needs onto `predicates`.
    fn restate_body(
        &self,
        ctx: &RestateCtx,
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
                    let expr = field.ty.restate_expr(
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
                        field.ty.restate_expr(
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
                    "`align` requires `unsafe_transmute = true`",
                ));
            } else {
                Transmute::Off
            };

        Ok(TypeStateArgs { state, transmute })
    }
}

/// Whether `#[typestate]` also derives the in-place `transmute_state`.
pub(crate) enum Transmute {
    /// Default, only `WithState` and `restate_with`.
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
                    "`{state}` must be one of the generic type \
                     parameters of `{}`",
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
                    "`#[typestate]` needs `state = <Ident>` unless the \
                     struct has exactly one generic type parameter",
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
                    "`#[typestate]` cannot generate `restate_with` for a \
                     struct that projects both `{state}::{first}` and \
                     `{state}::{found}`: the conversion `f` would be \
                     ambiguous"
                ),
            ));
        }

        Ok(Some(first))
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
                        "`unsafe_transmute = true` needs every field to \
                         be a bare `{state}::Assoc` projection, a ZST, \
                         or independent of `{state}`; use `restate_with` \
                         for fields that wrap the state"
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
                    "`unsafe_transmute = true` derives \
                     `TransmutableState` for this struct, which needs a \
                     guaranteed field layout; add `C` to its \
                     `#[repr(..)]`",
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
    /// target state. Pushes the `Restate` bounds it needs onto
    /// `predicates`.
    fn restate_expr(
        &self,
        ctx: &RestateCtx,
        expr: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        match self.restate_shape(ctx.state) {
            RestateShape::Unchanged => expr,
            RestateShape::Direct => {
                let f = ctx.f;
                quote!(#f(#expr))
            }
            RestateShape::Zst(value) => value,
            RestateShape::Tuple(tuple) => {
                tuple.restate_tuple(ctx, expr, predicates)
            }
            RestateShape::Wrapped(inner) => {
                let RestateCtx {
                    state,
                    target_state,
                    elem,
                    ..
                } = ctx;
                let inner_expr =
                    inner.restate_expr(ctx, quote!(#elem), predicates);
                self.restate_call(
                    expr,
                    quote!(#inner),
                    inner.with_ident_renamed(state, target_state),
                    quote!(|#elem| #inner_expr),
                    ctx,
                    predicates,
                )
            }
            RestateShape::Opaque => {
                let RestateCtx {
                    state,
                    target_state,
                    projection,
                    f,
                    ..
                } = ctx;
                self.restate_call(
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

    /// How `restate_expr` rebuilds a value of this type.
    fn restate_shape(&self, state: &Ident) -> RestateShape<'_> {
        if !self.mentions_ident(state) {
            return RestateShape::Unchanged;
        }
        if self.state_projection(state).is_some() {
            return RestateShape::Direct;
        }
        if let Some(value) = self.zst_value() {
            return RestateShape::Zst(value);
        }
        if let Type::Tuple(tuple) = self.peeled() {
            return RestateShape::Tuple(tuple);
        }

        self.restate_inner(state)
            .map_or(RestateShape::Opaque, RestateShape::Wrapped)
    }

    /// The one generic argument that mentions the state, or the element
    /// of an array.
    ///
    /// `Option<[S::Value; 2]> -> [S::Value; 2]`
    fn restate_inner(&self, state: &Ident) -> Option<&Type> {
        let path = match self.peeled() {
            Type::Array(array) => return Some(&array.elem),
            Type::Path(TypePath {
                qself: None, path, ..
            }) => path,
            _ => return None,
        };
        let PathArguments::AngleBracketed(args) =
            &path.segments.last()?.arguments
        else {
            return None;
        };

        let mut inners = args.args.iter().filter_map(|arg| {
            let GenericArgument::Type(ty) = arg else {
                return None;
            };
            ty.mentions_ident(state).then_some(ty)
        });
        let first = inners.next()?;
        inners.all(|ty| ty == first).then_some(first)
    }

    /// A `Restate<src, dst>` call on `expr` with the closure `convert`,
    /// and the bound it needs.
    fn restate_call(
        &self,
        expr: TokenStream,
        src: TokenStream,
        dst: TokenStream,
        convert: TokenStream,
        ctx: &RestateCtx,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        let target_ty =
            self.with_ident_renamed(ctx.state, ctx.target_state);

        predicates.push(parse_quote!(
            #self: ::groupoid::Restate<#src, #dst, Output = #target_ty>
        ));
        quote!(<#self as ::groupoid::Restate<#src, #dst>>::restate(#expr, &mut #convert))
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
    fn restate_tuple(
        &self,
        ctx: &RestateCtx,
        expr: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        let bindings: Vec<Ident> = (0..self.elems.len())
            .map(crate::naming::tuple_binding_ident)
            .collect();
        let elems =
            self.elems.iter().zip(&bindings).map(|(ty, binding)| {
                ty.restate_expr(ctx, quote!(#binding), predicates)
            });
        quote!({
            let (#(#bindings,)*) = #expr;
            (#(#elems,)*)
        })
    }
}

/// How `Type::restate_expr` rebuilds a value for the target state.
enum RestateShape<'a> {
    /// A type that doesn't mention the state, moved as is.
    Unchanged,
    /// `S::Value`, passed to `f`.
    Direct,
    /// A ZST, created fresh.
    Zst(TokenStream),
    /// `(T, ..)`
    Tuple(&'a TypeTuple),
    /// `Wrapper<T>` or `[T; N]`, one `Restate` layer around `T`.
    Wrapped(&'a Type),
    /// Anything else, one `Restate` call on the projection.
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

/// What `Type::restate_expr` needs to convert a projection.
struct RestateCtx<'a> {
    state: &'a Ident,
    target_state: &'a Ident,
    /// `Value` in `S::Value`.
    projection: &'a Ident,
    /// The conversion closure.
    f: &'a Ident,
    /// The closure binding for one element of a mapped or boxed value.
    elem: &'a Ident,
}
