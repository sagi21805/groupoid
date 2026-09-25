use extend::ext;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, ConstParam, Field, Fields, GenericParam, Generics, Ident, Index, ItemStruct,
    LifetimeParam, LitBool, LitInt, Path, PathArguments, Token, Type, TypeParam, TypePath,
    TypeTuple, WherePredicate,
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
    state: Ident,
    /// Whether the in-place `transmute_state` path is derived too, and
    /// how its alignment is pinned if so.
    transmute: Transmute,
    /// The one associated type the fields project through the state
    /// (`Value` in `S::Value`), if any: the leaf the by-value
    /// `restate_with` method converts.
    projection: Option<Ident>,
}

impl TypeState {
    pub(crate) fn new(args: TypeStateArgs, mut item_struct: ItemStruct) -> syn::Result<Self> {
        let (state, transmute) = args.resolve_state_and_transmute(&mut item_struct)?;

        state.bounds.push(parse_quote!(::groupoid::State));

        let state_ident = state.ident.clone();

        let projection = item_struct.unique_projection(&state_ident)?;

        if let Transmute::On(align) = &transmute {
            item_struct.require_transmutable_layout(&state_ident)?;
            if projection.is_none() {
                return Err(syn::Error::new_spanned(
                    &item_struct.ident,
                    format!(
                        "`unsafe_transmute = true` needs at least one field to be a bare \
                         `{state_ident}::Assoc` projection: without one there is nothing to \
                         transmute between states",
                    ),
                ));
            }
            item_struct.ensure_repr(align)?;
        }

        Ok(TypeState {
            item_struct,
            state: state_ident,
            transmute,
            projection,
        })
    }

    pub(crate) fn generate_typestate_impls(&self) -> TokenStream {
        let item_struct = &self.item_struct;
        let with_state_impl = self.with_state_impl();

        let (sized_with_state_impl, transmutable_state_impl) = match &self.transmute {
            Transmute::On(align) => (
                Some(self.sized_with_state_impl(align)),
                Some(self.transmutable_state_impl(align)),
            ),
            Transmute::Off => (None, None),
        };
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
        let state_ident = &self.state;
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

        let mut generics = self.item_struct.generics.clone();
        let align_arg = generics.pin_states_to_shared_layout(&[&self.state], align);

        let (impl_generics, _, where_clause) = generics.split_for_impl();

        quote! {
            impl #impl_generics ::groupoid::SizedWithState<__GROUPOID_SIZE, #align_arg>
                for #struct_ident #ty_generics #where_clause {}
        }
    }

    /// Implement `TransmutableState<S2, N, A>` for `Struct<S>` with
    /// `Target = Struct<S2>`, generic over *every* other state `S2` that
    /// could be plugged in (given the same bounds as `S`) at the same
    /// layout.
    fn transmutable_state_impl(&self, align: &Alignment) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let target_state = format_ident!("__GroupoidTargetState");
        let target_ty = self.target_ty(&target_state);

        let mut header_generics = self.item_struct.generics.clone();
        header_generics
            .params
            .push(self.target_state_generic_param(&target_state));

        let align_arg =
            header_generics.pin_states_to_shared_layout(&[&self.state, &target_state], align);

        let (impl_generics, _, where_clause) = header_generics.split_for_impl();
        let (_, ty_generics, _) = self.item_struct.generics.split_for_impl();

        quote! {
            // SAFETY: `Self` and `Target` are the same struct with only the state parameter swapped.
            // Every field is either a bare projection through that state, a ZST, or state-independent.
            // Both projections are pinned to the same size and alignment so `repr(C)` lays the two out identically.
            unsafe impl #impl_generics ::groupoid::TransmutableState<
                #target_state, __GROUPOID_SIZE, #align_arg
            > for #struct_ident #ty_generics #where_clause {
                type Target = #target_ty;
            }
        }
    }

    /// The inherent `restate_with` method: a by-value transition that
    /// rebuilds the struct field by field, so it exists whether or not the
    /// two states share a layout.
    ///
    /// The leaf conversion (`S::Value -> S2::Value`) is always taken from
    /// the caller; bit-reinterpreting between states is left to
    /// `TransmutableState` alone.
    fn restate_impl(&self, projection: &Ident) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let state = &self.state;
        let target_state = &format_ident!("__GroupoidTargetState");
        let leaf = &format_ident!("__groupoid_leaf");

        let target_ty = self.target_ty(target_state);
        let (impl_generics, ty_generics, where_clause) = self.item_struct.generics.split_for_impl();

        let cx = RestateLeaf {
            state,
            target_state,
            projection,
            leaf,
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

        // Method-level generics: the target state, with `S`'s bounds.
        let mut with_generics = Generics::default();
        with_generics
            .params
            .push(self.target_state_generic_param(&target_state));
        with_generics
            .make_where_clause()
            .predicates
            .extend(predicates);

        let (with_impl_generics, _, with_where_clause) = with_generics.split_for_impl();

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
                    mut #leaf: impl FnMut(#state::#projection)
                        -> #target_state::#projection,
                ) -> #target_ty #with_where_clause {
                    #body
                }
            }
        }
    }

    /// A `target_state` GenericParam carrying the state parameter's bounds.
    fn target_state_generic_param(&self, target_state: &Ident) -> GenericParam {
        let state_param = self
            .item_struct
            .generics
            .type_params()
            .find(|param| param.ident == self.state)
            .expect("the state ident was checked to be a type param in `new`");

        GenericParam::Type(TypeParam {
            ident: target_state.clone(),
            ..state_param.clone()
        })
    }

    /// The struct's type with the state parameter swapped for `target_state`.
    ///
    /// `Wrap<S> -> Wrap<target_state>`
    fn target_ty(&self, target_state: &Ident) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let args = self
            .item_struct
            .generics
            .params
            .iter()
            .map(|param| match param {
                GenericParam::Type(param) if param.ident == self.state => {
                    quote!(#target_state)
                }
                GenericParam::Type(TypeParam { ident, .. })
                | GenericParam::Const(ConstParam { ident, .. }) => quote!(#ident),
                GenericParam::Lifetime(LifetimeParam { lifetime, .. }) => quote!(#lifetime),
            });

        quote!(#struct_ident<#(#args),*>)
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
            align: align.map_or(Alignment::Inferred, Alignment::Forced),
        })
    }
}

impl TypeStateArgs {
    /// Find the state ident, and the alignment of the struct.
    fn resolve_state_and_transmute(
        self,
        item_struct: &mut ItemStruct,
    ) -> syn::Result<(&mut TypeParam, Transmute)> {
        let state = match &self.state {
            Some(ident) => item_struct.lookup_state_param(ident)?,
            None => item_struct.infer_state_ident()?,
        };
        if !self.unsafe_transmute {
            // By value `restate_with` never looks at alignment, so an
            // `align` that pins nothing is a mistake
            if let Alignment::Forced(n) = self.align {
                return Err(syn::Error::new(
                    n.span(),
                    "`align` only applies with `unsafe_transmute = true`. The by-value \
                     `restate_with` path never looks at alignment",
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
    /// Default, only `WithState` and `restate_with`.
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

#[ext]
impl ItemStruct {
    /// Infer the struct's state ident from its generic type parameters,
    /// erroring if it has more than one or none.
    fn infer_state_ident(&mut self) -> syn::Result<&mut TypeParam> {
        let mut type_params = self.generics.type_params_mut();

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
    /// (`Value` in `S::Value`), at any depth.
    fn unique_projection(&self, state_ident: &Ident) -> syn::Result<Option<Ident>> {
        let mut projections = self
            .fields
            .iter()
            .flat_map(|field| field.ty.projections(state_ident));

        let Some(first) = projections.next() else {
            return Ok(None);
        };
        if let Some(found) = projections.find(|found| *found != first) {
            return Err(syn::Error::new(
                found.span(),
                format!(
                    "`#[typestate]` cannot generate `restate_with` for a struct that projects \
                     both `{state_ident}::{first}` and `{state_ident}::{found}`: the leaf \
                     conversion would be ambiguous"
                ),
            ));
        }

        Ok(Some(first))
    }

    /// Checks that every field's shape lets the container's layout be
    /// pinned by the state's `SizedGroup`/`AlignedGroup` impls, so
    /// `SizedWithState` and `TransmutableState` mean something
    fn require_transmutable_layout(&self, state_ident: &Ident) -> syn::Result<()> {
        for field in self.fields.iter() {
            if !field.is_transmutable(state_ident) {
                return Err(syn::Error::new_spanned(
                    &field.ty,
                    format!(
                        "`unsafe_transmute = true` needs every field to be a bare \
                         `{state_ident}::Assoc` projection, a ZST, or independent of \
                         `{state_ident}`; this one reaches the state through a wrapper, so the \
                         two states' layouts cannot be pinned to each other - use `restate_with` \
                         instead"
                    ),
                ));
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

        if let Alignment::Forced(n) = align {
            self.attrs.push(parse_quote!(#[repr(align(#n))]));
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
    /// Requires all `states` to share one size via `SizedGroup`, and one
    /// alignment via `AlignedGroup` unless it is forced. Returns the
    /// alignment to use.
    fn pin_states_to_shared_layout(&mut self, states: &[&Ident], align: &Alignment) -> TokenStream {
        self.params.push(parse_quote!(const __GROUPOID_SIZE: usize));

        let predicates = &mut self.make_where_clause().predicates;
        for state in states {
            predicates.push(parse_quote!(
                #state::Marker: ::groupoid::SizedGroup<__GROUPOID_SIZE>
            ));
        }

        if let Alignment::Forced(n) = align {
            return quote!(#n);
        }
        self.params
            .push(parse_quote!(const __GROUPOID_ALIGN: usize));

        let predicates = &mut self.make_where_clause().predicates;
        for state in states {
            predicates.push(parse_quote!(
                #state::Marker: ::groupoid::AlignedGroup<__GROUPOID_ALIGN>
            ));
        }

        quote!(__GROUPOID_ALIGN)
    }
}

#[ext]
impl Field {
    /// Whether this field's layout can be pinned across states: a bare
    /// `S::Value` projection, a ZST (`()`, `PhantomData<..>`, etc.), or a
    /// type that never mentions the state. A field reaching the state
    /// through a wrapper (`Option<S::Value>`, `[S::Value; 2]`, etc.) is not.
    fn is_transmutable(&self, state_ident: &Ident) -> bool {
        self.ty.state_projection(state_ident).is_some()
            || self.ty.is_zst()
            || !self.ty.mentions_ident(state_ident)
    }
}

#[ext]
impl Type {
    /// Every `Assoc` of an `S::Assoc` written anywhere inside this type, so
    /// `Option<S::Value>` yields `Value` just as a bare `S::Value` does.
    fn projections(&self, state_ident: &Ident) -> Vec<Ident> {
        let mut projections = Projections {
            state_ident,
            found: Vec::new(),
        };
        projections.visit_type(self);
        projections.found
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
        ctx: &RestateLeaf,
        src: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        match self.restate_shape(ctx.state) {
            RestateShape::Unchanged => src,
            RestateShape::Leaf => {
                let leaf = ctx.leaf;
                quote!(#leaf(#src))
            }
            RestateShape::Zst(value) => value,
            RestateShape::Tuple(tuple) => tuple.restate_expr(ctx, src, predicates),
            RestateShape::Mapped(elem) => elem.restate_mapped_expr(ctx, src, predicates),
            RestateShape::Boxed(inner) => inner.restate_boxed_expr(ctx, src, predicates),
            RestateShape::Opaque => self.opaque_restate_expr(ctx, src, predicates),
        }
    }

    /// How `restate_expr` rebuilds a value of this type.
    fn restate_shape(&self, state_ident: &Ident) -> RestateShape<'_> {
        if !self.mentions_ident(state_ident) {
            return RestateShape::Unchanged;
        }
        if self.state_projection(state_ident).is_some() {
            return RestateShape::Leaf;
        }
        if let Some(value) = self.zst_value() {
            return RestateShape::Zst(value);
        }

        match self.peeled() {
            Type::Tuple(tuple) => RestateShape::Tuple(tuple),
            Type::Array(array) => RestateShape::Mapped(&array.elem),
            Type::Path(TypePath {
                qself: None, path, ..
            }) => {
                if let Some(inner) = path.std_item_arg("option", "Option") {
                    RestateShape::Mapped(inner)
                } else if let Some(inner) = path.std_item_arg("boxed", "Box") {
                    RestateShape::Boxed(inner)
                } else {
                    RestateShape::Opaque
                }
            }
            _ => RestateShape::Opaque,
        }
    }

    /// `restate_expr` for a container of this type with a by-value `map`
    /// (`[Self; N]`, `Option<Self>`): each element is rebuilt inside the
    /// closure.
    fn restate_mapped_expr(
        &self,
        ctx: &RestateLeaf,
        src: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        let elem = format_ident!("__groupoid_elem");
        let inner = self.restate_expr(ctx, quote!(#elem), predicates);
        quote!(#src.map(|#elem| #inner))
    }

    /// `restate_expr` for a `Box<Self>`: the value is moved out, rebuilt,
    /// and boxed again.
    ///
    /// Reallocates: a pointer cast would only be sound when the inner
    /// conversion is itself in place, which `restate_with` and nested
    /// `Option`s are not.
    fn restate_boxed_expr(
        &self,
        ctx: &RestateLeaf,
        src: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        let elem = format_ident!("__groupoid_elem");
        let inner = self.restate_expr(ctx, quote!(#elem), predicates);
        quote!(::std::boxed::Box::new({
            let #elem = *#src;
            #inner
        }))
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
            state,
            target_state,
            projection,
            leaf,
        } = cx;
        let target_ty = self.with_ident_renamed(state, target_state);
        let (src_leaf, dst_leaf) = (
            quote!(#state::#projection),
            quote!(#target_state::#projection),
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

#[ext]
impl TypeTuple {
    /// `Type::restate_expr` for a tuple: `src` is destructured and rebuilt
    /// element by element.
    fn restate_expr(
        &self,
        ctx: &RestateLeaf,
        src: TokenStream,
        predicates: &mut Vec<WherePredicate>,
    ) -> TokenStream {
        let bindings: Vec<Ident> = (0..self.elems.len())
            .map(|i| format_ident!("__groupoid_elem{i}"))
            .collect();
        let elems = self
            .elems
            .iter()
            .zip(&bindings)
            .map(|(ty, binding)| ty.restate_expr(ctx, quote!(#binding), predicates));
        quote!({
            let (#(#bindings,)*) = #src;
            (#(#elems,)*)
        })
    }
}

/// How `Type::restate_expr` rebuilds a value of some type for the target
/// state.
enum RestateShape<'a> {
    /// The type never mentions the state: the value is moved as it is.
    Unchanged,
    /// A bare `S::Value`: converted by the caller's leaf closure.
    Leaf,
    /// A ZST: a fresh value of it is written out.
    Zst(TokenStream),
    /// A tuple: rebuilt element by element.
    Tuple(&'a TypeTuple),
    /// `[T; N]` or `Option<T>`, holding the `T`: rebuilt with `map`.
    Mapped(&'a Type),
    /// `Box<T>`, holding the `T`: unboxed, rebuilt, and boxed again.
    Boxed(&'a Type),
    /// Anything else: handed to the user's `groupoid::Restate` impl.
    Opaque,
}

/// Collects the `Assoc` of every `S::Assoc` among the types it visits,
/// for `Type::projections`.
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

/// Information to restate certain state.
struct RestateLeaf<'a> {
    state: &'a Ident,
    target_state: &'a Ident,
    projection: &'a Ident,
    leaf: &'a Ident,
}
