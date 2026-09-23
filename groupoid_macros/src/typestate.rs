use extend::ext;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Fields, GenericParam, Generics, Ident, Index, ItemStruct, LitBool, LitInt, Path,
    PathArguments, Token, Type, TypeParam, TypePath, WherePredicate,
    parse::{Parse, ParseStream},
    parse_quote,
    visit::{self, Visit},
};

use crate::syn_ext::{
    AttributeExt as _, GenericsExt as _, OptionExt as _, PathExt as _, TypeExt as _,
};

pub(crate) struct TypeState {
    /// The original struct with the state parameter's bounds extended
    /// with `::groupoid::State`, and its `repr` settled when the in-place
    /// path is on.
    item_struct: ItemStruct,
    /// The generic type parameter carrying the state.
    state_ident: Ident,
    /// Whether the in-place `transmute_state` path is derived too, and
    /// how its alignment is pinned if so.
    transmute: Transmute,
    /// The one associated type the fields project through the state
    /// (`Value` in `S::Value`), if any: the leaf the by-value `restate`
    /// methods convert.
    projection: Option<Ident>,
}

impl TypeState {
    pub(crate) fn new(args: TypeStateArgs, mut item_struct: ItemStruct) -> syn::Result<Self> {
        let (state_ident, transmute) = args.resolve_state_and_transmute(&item_struct)?;
        let projection = item_struct.unique_projection(&state_ident)?;

        if let Transmute::On(align) = &transmute {
            item_struct.require_transmutable_layout(&state_ident)?;
            item_struct.ensure_repr(align)?;
        }
        item_struct
            .lookup_state_param(&state_ident)?
            .bounds
            .push(parse_quote!(::groupoid::State));

        Ok(TypeState {
            item_struct,
            state_ident,
            transmute,
            projection,
        })
    }

    pub(crate) fn generate_typestate_impls(&self) -> TokenStream {
        let item_struct = &self.item_struct;
        let with_state_impl = self.with_state_impl();

        // Whatever bound on the state param grants the field's projection
        // (required for the field to type-check in the first place) is
        // assumed to also grant `Marker`; if it turns out not to,
        // rustc rejects the generated impls with an ordinary
        // associated-type error.
        //
        // The in-place path only exists under `unsafe_transmute = true`,
        // which `new` has already validated the struct for.
        // `TransmutableState` links `Self` to *every other* state that
        // could be plugged into the same struct, at the same layout, so
        // it comes and goes together with `SizedWithState`.
        let align = match &self.transmute {
            Transmute::On(align) => Some(align),
            Transmute::Off => None,
        };
        let sized_with_state_impl = align.map(|align| self.sized_with_state_impl(align));
        let transmutable_state_impl = align.map(|align| self.transmutable_state_impl(align));
        // By value the container is rebuilt field by field, so its layout
        // never enters the picture: any struct that projects through the
        // state at all gets `restate`.
        let restate_impl = self
            .projection
            .as_ref()
            .map(|projection| self.restate_impl(projection));

        quote! {
            #item_struct

            #with_state_impl

            #sized_with_state_impl

            #transmutable_state_impl

            #restate_impl
        }
    }

    fn with_state_impl(&self) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let state_ident = &self.state_ident;
        // split_for_impl() correctly handles any number of generics +
        // where clauses
        let (impl_generics, ty_generics, where_clause) = self.item_struct.generics.split_for_impl();

        quote! {
            impl #impl_generics ::groupoid::WithState for #struct_ident #ty_generics #where_clause {
                type State = #state_ident;
            }
        }
    }

    /// Implement `SizedWithState<N, A>` if the type of the blueprint inside
    /// the struct is `#[size(N)]`, with `A` either the state's own
    /// alignment or the one forced by `align = A`.
    fn sized_with_state_impl(&self, align: &Alignment) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let (_, ty_generics, _) = self.item_struct.generics.split_for_impl();
        let align_arg = align.const_generic_arg();

        let mut sized_generics = self.item_struct.generics.clone();
        sized_generics.pin_states_to_shared_layout(&[&self.state_ident], align);

        let (sized_impl_generics, _, sized_where_clause) = sized_generics.split_for_impl();

        quote! {
            impl #sized_impl_generics ::groupoid::SizedWithState<__GROUPOID_SIZE, #align_arg>
                for #struct_ident #ty_generics #sized_where_clause {}
        }
    }

    /// Implement `TransmutableState<S2, N, A>` for `Struct<S>` with
    /// `Target = Struct<S2>`, generic over *every* other state `S2` that
    /// could be plugged in (given the same bounds as `S`) at the same
    /// layout.
    fn transmutable_state_impl(&self, align: &Alignment) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let target_state_ident = format_ident!("__GroupoidTargetState");
        let align_arg = align.const_generic_arg();

        let (target_state_param, target_generics) = self.target_generics(&target_state_ident);

        // The impl header needs both states in scope at once, hence the
        // `<S, __GroupoidTargetState, ..>` parameter list.
        let mut header_generics = self.item_struct.generics.clone();
        header_generics
            .params
            .push(GenericParam::Type(target_state_param));

        // Both states are then pinned to the *same* layout, which is the
        // safety argument for the `unsafe impl` spelled out in
        // full below.
        header_generics
            .pin_states_to_shared_layout(&[&self.state_ident, &target_state_ident], align);

        let (impl_generics, _, where_clause) = header_generics.split_for_impl();
        // `Wrap<__GroupoidTargetState>` (the `Target` associated type) ...
        let (_, target_ty_generics, _) = target_generics.split_for_impl();
        // ... and `Wrap<S>` (the `for` type).
        let (_, self_ty_generics, _) = self.item_struct.generics.split_for_impl();

        quote! {
            // SAFETY: `Self` and `Target` are literally the same struct with
            // only the state parameter swapped. Every field is either a bare
            // projection through that state, a ZST, or state-independent, and
            // both sides' projections are pinned to the same size and alignment
            // above - so `repr(C)` lays the two out identically.
            unsafe impl #impl_generics ::groupoid::TransmutableState<
                #target_state_ident, __GROUPOID_SIZE, #align_arg
            > for #struct_ident #self_ty_generics #where_clause {
                type Target = #struct_ident #target_ty_generics;
            }
        }
    }

    /// The inherent `restate_with` / `restate` methods: by-value transitions
    /// that rebuild the struct field by field, so they exist whether or not
    /// the two states share a layout.
    ///
    /// The leaf conversion (`S::Value -> S2::Value`) is the one step the
    /// macro cannot make safe on its own - the `#[size]` pin proves size,
    /// not bit validity - so it is either taken from the caller
    /// (`restate_with`) or transmuted under the shared `SizedGroup` pin
    /// (`unsafe fn restate`), with a single generated body between them.
    /// Alignment is irrelevant by value, so `restate` is bounded on size
    /// alone in both `Alignment` modes.
    fn restate_impl(&self, projection: &Ident) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let state_ident = &self.state_ident;
        let target_state_ident = format_ident!("__GroupoidTargetState");
        let leaf = format_ident!("__groupoid_leaf");

        let (target_state_param, target_generics) = self.target_generics(&target_state_ident);
        let (_, target_ty_generics, _) = target_generics.split_for_impl();
        let (impl_generics, ty_generics, where_clause) = self.item_struct.generics.split_for_impl();

        // Every field is moved out of `self` and rebuilt for the target
        // state; wrappers the recursion cannot see through add a
        // `Restate` bound to `predicates`.
        let cx = RestateLeaf {
            state_ident,
            target_state_ident: &target_state_ident,
            projection,
            leaf: &leaf,
        };
        let mut predicates = Vec::new();
        let body = match &self.item_struct.fields {
            Fields::Named(named) => {
                let fields = named.named.iter().map(|field| {
                    let ident = field.ident.as_ref().expect("named fields have identifiers");
                    let expr = field
                        .ty
                        .restate_expr(&cx, quote!(self.#ident), &mut predicates);
                    quote!(#ident: #expr)
                });
                quote!(#struct_ident { #(#fields),* })
            }
            Fields::Unnamed(unnamed) => {
                let fields = unnamed.unnamed.iter().enumerate().map(|(i, field)| {
                    let index = Index::from(i);
                    field
                        .ty
                        .restate_expr(&cx, quote!(self.#index), &mut predicates)
                });
                quote!(#struct_ident(#(#fields),*))
            }
            Fields::Unit => unreachable!("a unit struct has no field to project through the state"),
        };

        // Method-level generics: the target state (with `S`'s bounds) for
        // both methods, plus the shared size pin for `restate`.
        let mut with_generics = Generics::default();
        with_generics
            .params
            .push(GenericParam::Type(target_state_param));
        let mut restate_generics = with_generics.clone();
        restate_generics.pin_states_to_shared_size(&[state_ident, &target_state_ident]);
        with_generics
            .make_where_clause()
            .predicates
            .extend(predicates.iter().cloned());
        restate_generics
            .make_where_clause()
            .predicates
            .extend(predicates);

        let (with_impl_generics, _, with_where_clause) = with_generics.split_for_impl();
        let (restate_impl_generics, _, restate_where_clause) = restate_generics.split_for_impl();

        quote! {
            impl #impl_generics #struct_ident #ty_generics #where_clause {
                /// Rebuilds `self` for the target state by value, converting
                /// every projection through the state with `leaf`.
                ///
                /// A projection is reached through any nesting of `Option`,
                /// arrays, `Box` and tuples; any other wrapper around one is
                /// handed to `groupoid::Restate`. Fields that do not
                /// mention the state are moved as they are.
                pub fn restate_with #with_impl_generics (
                    self,
                    mut #leaf: impl FnMut(#state_ident::#projection)
                        -> #target_state_ident::#projection,
                ) -> #struct_ident #target_ty_generics #with_where_clause {
                    #body
                }

                /// Rebuilds `self` for the target state by value,
                /// bit-reinterpreting every projection through the state
                /// under the shared `SizedGroup<SIZE>` pin.
                ///
                /// # Safety
                ///
                /// The bit pattern of every projection inside `self` must be
                /// a valid value of the target state's projection.
                pub unsafe fn restate #restate_impl_generics (
                    self,
                ) -> #struct_ident #target_ty_generics #restate_where_clause {
                    self.restate_with::<#target_state_ident>(|value| unsafe {
                        ::groupoid::restate_leaf::<_, _, __GROUPOID_SIZE>(value)
                    })
                }
            }
        }
    }

    /// Renaming the state param inside a clone of the struct's generics
    /// produces both halves of the target side at once: the
    /// `Wrap<__GroupoidTargetState>` type arguments, and a
    /// `__GroupoidTargetState` parameter that already carries `S`'s bounds
    /// (`Meta + ::groupoid::State`). Both are returned, the parameter on its
    /// own so it can be pushed onto another generics list.
    fn target_generics(&self, target_state_ident: &Ident) -> (TypeParam, Generics) {
        let mut target_generics = self.item_struct.generics.clone();
        let target_state_param = target_generics
            .type_param_mut(&self.state_ident)
            .expect("the state ident was checked to be a type param in `new`");
        target_state_param.ident = target_state_ident.clone();
        let target_state_param = target_state_param.clone();

        (target_state_param, target_generics)
    }
}

/// Parses `#[typestate]`'s arguments, an optional `state = <Ident>`, an
/// optional `unsafe_transmute = <bool>` and an optional
/// `align = <integer literal>`, in any order, each at most once
pub(crate) struct TypeStateArgs {
    state: Option<Ident>,
    align: Alignment,
    unsafe_transmute: bool,
}

impl Parse for TypeStateArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Each argument is optional but may be given at most once
        let mut state: Option<Ident> = None;
        let mut unsafe_transmute: Option<LitBool> = None;
        let mut align: Option<LitInt> = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            match key.to_string().as_str() {
                "state" => state.parse_once(&key, input)?,
                "unsafe_transmute" => unsafe_transmute.parse_once(&key, input)?,
                "align" => align.parse_once(&key, input)?,
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        "expected `state = <Ident>`, `unsafe_transmute = <bool>` or `align = \
                         <integer literal>`",
                    ));
                }
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(TypeStateArgs {
            state,
            unsafe_transmute: unsafe_transmute.is_some_and(|flag| flag.value()),
            align: align.into(),
        })
    }
}

impl TypeStateArgs {
    /// Find the state ident, and the alignment of the struct.
    fn resolve_state_and_transmute(
        self,
        item_struct: &ItemStruct,
    ) -> syn::Result<(Ident, Transmute)> {
        let state = match self.state {
            Some(ident) => ident,
            None => item_struct.infer_state_ident()?.ident.clone(),
        };
        if !self.unsafe_transmute {
            // By value `restate` never looks at alignment, so an `align`
            // that pins nothing is a mistake
            if let Alignment::Forced(n) = self.align {
                return Err(syn::Error::new(
                    n.span(),
                    "`align` only applies with `unsafe_transmute = true`. The by-value `restate` \
                     path never looks at alignment",
                ));
            } else {
                return Ok((state, Transmute::Off));
            }
        }

        Ok((state, Transmute::On(self.align)))
    }
}

/// Whether `#[typestate]` also derives the `transmute_state`
pub(crate) enum Transmute {
    /// Default, only `WithState` and `restate` / `restate_with`.
    Off,
    /// `unsafe_transmute = true`, additionally implements `SizedWithState` and
    /// `TransmutableState`
    On(Alignment),
}

/// How the container's alignment is pinned.
pub(crate) enum Alignment {
    /// No `align` argument.
    Inferred,
    /// `align = N`. The container is forced to `#[repr(align(N))]`
    Forced(LitInt),
}

impl From<Option<LitInt>> for Alignment {
    /// An `align = N` argument, or its absence.
    fn from(align: Option<LitInt>) -> Self {
        match align {
            Some(n) => Alignment::Forced(n),
            None => Alignment::Inferred,
        }
    }
}

impl Alignment {
    /// What goes in the `ALIGN` slot of `SizedWithState` /
    /// `TransmutableState`.
    fn const_generic_arg(&self) -> TokenStream {
        match self {
            Alignment::Inferred => quote!(__GROUPOID_ALIGN),
            Alignment::Forced(n) => quote!(#n),
        }
    }

    /// The extra marker bound unifying the states' alignments, when the
    /// alignment is not forced.
    fn aligned_group_bound(&self) -> Option<TokenStream> {
        match self {
            Alignment::Inferred => Some(quote!(::groupoid::AlignedGroup<__GROUPOID_ALIGN>)),
            Alignment::Forced(_) => None,
        }
    }

    /// The `#[repr(align(N))]` to add to the struct, when forced.
    fn repr_align_attr(&self) -> Option<Attribute> {
        match self {
            Alignment::Inferred => None,
            Alignment::Forced(n) => Some(parse_quote!(#[repr(align(#n))])),
        }
    }
}

#[ext]
impl ItemStruct {
    /// Infer the struct's state ident from its generic type parameters,
    /// erroring if it has more than one or none.
    fn infer_state_ident(&self) -> syn::Result<&TypeParam> {
        let mut type_params = self.generics.type_params();

        // The first type parameter, provided there is no second.
        type_params
            .next()
            .filter(|_| type_params.next().is_none())
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    &self.ident,
                    "`#[typestate]` needs `state = <Ident>` unless the struct has exactly one \
                     generic type parameter",
                )
            })
    }

    /// The one associated type the fields project through the state
    /// (`Value` in `S::Value`)
    fn unique_projection(&self, state_ident: &Ident) -> syn::Result<Option<Ident>> {
        let mut projection: Option<Ident> = None;

        for field in self.fields.iter() {
            for found in field.ty.projections_through(state_ident) {
                match &projection {
                    None => projection = Some(found),
                    Some(first) if *first == found => {}
                    Some(first) => {
                        return Err(syn::Error::new(
                            found.span(),
                            format!(
                                "`#[typestate]` cannot generate `restate` for a struct that \
                                 projects both `{state_ident}::{first}` and \
                                 `{state_ident}::{found}`: the leaf conversion would be ambiguous"
                            ),
                        ));
                    }
                }
            }
        }

        Ok(projection)
    }

    /// Checks that every field's shape lets the container's layout be
    /// pinned by the state's `SizedGroup`/`AlignedGroup` impls, so
    /// `SizedWithState` and `TransmutableState` mean something
    fn require_transmutable_layout(&self, state_ident: &Ident) -> syn::Result<()> {
        for field in self.fields.iter() {
            match field.ty.shape(state_ident) {
                FieldShape::Zst | FieldShape::Independent | FieldShape::Projection => {}
                FieldShape::Wrapped => {
                    return Err(syn::Error::new_spanned(
                        &field.ty,
                        format!(
                            "`unsafe_transmute = true` needs every field to be a bare \
                             `{state_ident}::Assoc` projection, a ZST, or independent of \
                             `{state_ident}`; this one reaches the state through a wrapper, so \
                             the two states' layouts cannot be pinned to each other - use \
                             `restate` / `restate_with` instead"
                        ),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Settles the struct's `repr`.
    ///
    /// This method will ensure that this struct layout is guaranteed with
    /// `#[repr(C | transparent)]` and will ensure a minimal alignment.
    fn ensure_repr(&mut self, align: &Alignment) -> syn::Result<()> {
        let reprs: Vec<&Attribute> = self
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("repr"))
            .collect();

        match reprs.as_slice() {
            [] => self.attrs.push(parse_quote!(#[repr(C)])),
            [first, ..] if !reprs.iter().any(|attr| attr.guarantees_layout()) => {
                return Err(syn::Error::new_spanned(
                    first,
                    "`unsafe_transmute = true` derives `TransmutableState` for this struct, which \
                     needs a guaranteed field layout; add `C` to its `#[repr(..)]`",
                ));
            }
            _ => {}
        }

        if let Some(attr) = align.repr_align_attr() {
            self.attrs.push(attr);
        }

        Ok(())
    }

    /// Looks up `state_ident` among the struct's generic type parameters,
    /// erroring if it isn't one of them.
    fn lookup_state_param(&mut self, state_ident: &Ident) -> syn::Result<&mut TypeParam> {
        let msg = format!(
            "`{}` must be one of the generic type parameters of `{}`",
            state_ident, self.ident
        );
        self.generics
            .type_param_mut(state_ident)
            .ok_or_else(|| syn::Error::new(state_ident.span(), msg))
    }
}

#[ext(name = GenericsExt)]
impl Generics {
    /// Adds the synthetic const parameters and the predicates pinning
    /// every state's layout to them.
    ///
    /// `__GROUPOID_SIZE` is always added. `__GROUPOID_ALIGN` only joins it
    /// when the alignment is inferred; a forced alignment puts a
    /// literal in the `ALIGN` slot instead, so there is no parameter
    /// to bind and the states' own alignments are left free.
    fn pin_states_to_shared_layout(&mut self, states: &[&Ident], align: &Alignment) {
        self.pin_states_to_shared_size(states);

        let Some(bound) = align.aligned_group_bound() else {
            return;
        };
        self.params
            .push(parse_quote!(const __GROUPOID_ALIGN: usize));

        let predicates = &mut self.make_where_clause().predicates;
        for state in states {
            predicates.push(parse_quote!(#state::Marker: #bound));
        }
    }

    /// Adds the synthetic `__GROUPOID_SIZE` const parameter and the
    /// predicates pinning every state's size to it.
    fn pin_states_to_shared_size(&mut self, states: &[&Ident]) {
        self.params.push(parse_quote!(const __GROUPOID_SIZE: usize));

        let predicates = &mut self.make_where_clause().predicates;
        for state in states {
            predicates.push(parse_quote!(
                #state::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>
            ));
        }
    }
}

#[ext]
impl Type {
    /// Every `Assoc` of an `S::Assoc` written anywhere inside this type, so
    /// `Option<S::Value>` yields `Value` just as a bare `S::Value` does.
    fn projections_through(&self, state_ident: &Ident) -> Vec<Ident> {
        let mut projections = Projections {
            state_ident,
            found: Vec::new(),
        };
        projections.visit_type(self);
        projections.found
    }

    /// Classifies this type for `require_transmutable_layout`.
    fn shape(&self, state_ident: &Ident) -> FieldShape {
        if self.state_projection(state_ident).is_some() {
            FieldShape::Projection
        } else if self.is_zst() {
            FieldShape::Zst
        } else if self.mentions_ident(state_ident) {
            FieldShape::Wrapped
        } else {
            FieldShape::Independent
        }
    }

    /// The expression rebuilding a value of this type for the target
    /// state, given `src`, an expression of this type that may be moved.
    ///
    /// Recurses through the shapes it can see (`Option`, arrays, `Box`,
    /// tuples, parentheses); anything else that mentions the state is
    /// handed to `groupoid::Restate`, and the bound that needs is pushed
    /// onto `predicates` so a missing impl surfaces at the call site rather
    /// than inside the generated method.
    fn restate_expr(
        &self,
        cx: &RestateLeaf,
        src: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        if !self.mentions_ident(cx.state_ident) {
            return src;
        }
        if self.state_projection(cx.state_ident).is_some() {
            let leaf = cx.leaf;
            return quote!(#leaf(#src));
        }
        if let Some(value) = self.zst_value() {
            return value;
        }

        let elem = format_ident!("__groupoid_elem");
        match self.peeled() {
            Type::Tuple(tuple) => {
                let bindings: Vec<Ident> = (0..tuple.elems.len())
                    .map(|i| format_ident!("__groupoid_elem{i}"))
                    .collect();
                let elems = tuple
                    .elems
                    .iter()
                    .zip(&bindings)
                    .map(|(ty, binding)| ty.restate_expr(cx, quote!(#binding), predicates));
                quote!({
                    let (#(#bindings,)*) = #src;
                    (#(#elems,)*)
                })
            }
            Type::Array(array) => {
                let inner = array.elem.restate_expr(cx, quote!(#elem), predicates);
                quote!(#src.map(|#elem| #inner))
            }
            Type::Path(TypePath {
                qself: None, path, ..
            }) => {
                if let Some(inner) = path.std_item_arg("option", "Option") {
                    let inner = inner.restate_expr(cx, quote!(#elem), predicates);
                    quote!(#src.map(|#elem| #inner))
                } else if let Some(inner) = path.std_item_arg("boxed", "Box") {
                    // Reallocates: a pointer cast would only be sound when the
                    // inner conversion is itself in place, which
                    // `restate_with` and nested `Option`s are not.
                    let inner = inner.restate_expr(cx, quote!(#elem), predicates);
                    quote!(::std::boxed::Box::new({
                        let #elem = *#src;
                        #inner
                    }))
                } else {
                    self.opaque_restate_expr(cx, src, predicates)
                }
            }
            _ => self.opaque_restate_expr(cx, src, predicates),
        }
    }

    /// `restate_expr` for a type the recursion cannot see through: a call
    /// into the user's `Restate` impl, and the bound requiring it.
    fn opaque_restate_expr(
        &self,
        cx: &RestateLeaf,
        src: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        let RestateLeaf {
            state_ident,
            target_state_ident,
            projection,
            leaf,
        } = cx;
        let target_ty = self.with_ident_renamed(state_ident, target_state_ident);
        let (src_leaf, dst_leaf) = (
            quote!(#state_ident::#projection),
            quote!(#target_state_ident::#projection),
        );

        predicates.push(parse_quote!(
            #self: ::groupoid::Restate<#src_leaf, #dst_leaf, Output = #target_ty>
        ));
        quote!(<#self as ::groupoid::Restate<#src_leaf, #dst_leaf>>::restate(#src, &mut #leaf))
    }

    /// `Value` when this type is a state type like `S::Value`.
    fn state_projection(&self, state_ident: &Ident) -> Option<&Ident> {
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
            [state_seg, type_seg] => (state_seg.ident == *state_ident
                && state_seg.arguments == PathArguments::None
                && type_seg.arguments == PathArguments::None)
                .then_some(&type_seg.ident),
            _ => None,
        }
    }
}

/// Collects the `Assoc` of every `S::Assoc` among the types it visits,
/// for `projections_through`.
struct Projections<'a> {
    state_ident: &'a Ident,
    found: Vec<Ident>,
}

impl<'ast> Visit<'ast> for Projections<'_> {
    fn visit_type(&mut self, ty: &'ast Type) {
        match ty.state_projection(self.state_ident) {
            Some(assoc) => self.found.push(assoc.clone()),
            None => visit::visit_type(self, ty),
        }
    }
}

/// How a field's type relates to the state parameter
enum FieldShape {
    /// `S::Value`
    Projection,
    /// `()`, `PhantomData<..>`, etc.
    Zst,
    /// Mentions the state through something else `Option<S::Value>`,
    /// `[S::Value; 2]`, etc.
    Wrapped,
    /// Never mentions the state at all.
    Independent,
}

/// What `Type::restate_expr` needs to know about the transition it is
/// spelling: which parameter is the state, what it becomes, which
/// associated type is the leaf, and the binding holding the leaf
/// conversion.
struct RestateLeaf<'a> {
    state_ident: &'a Ident,
    target_state_ident: &'a Ident,
    projection: &'a Ident,
    leaf: &'a Ident,
}
