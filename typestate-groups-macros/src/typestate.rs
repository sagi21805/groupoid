use extend::ext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, ConstParam, Field, GenericParam, Generics, Ident,
    ItemStruct, LifetimeParam, LitBool, LitInt, Path, PathArguments,
    Token, Type, TypeParam, TypePath, TypeReference, WherePredicate,
    parse::{Parse, ParseStream},
    parse_quote,
    visit::{self, Visit},
};

use crate::syn_ext::{
    AttributeExt as _, GenericsExt as _, OptionExt as _, PathExt as _,
    TypeExt as _, WherePredicateExt as _,
};

pub(crate) struct TypeState {
    /// The struct with the state bounded by `State`.
    item_struct: ItemStruct,
    /// The generic type parameter carrying the state.
    state: Ident,
    /// The target state, with the state's inline bounds.
    target_state: TypeParam,
    /// `Wrap<S> -> Wrap<__TypestateGroupsTargetState>`
    target_ty: TokenStream,
    /// Whether `TransmutableState` and `CastableState` are derived too.
    transmute: Transmute,
    /// How each field changes between states, in field order. Empty
    /// without `unsafe_transmute = true`.
    shapes: Vec<FieldShape>,
}

impl TypeState {
    /// Resolves the state and validates the fields.
    pub(crate) fn new(
        args: TypeStateArgs,
        mut item_struct: ItemStruct,
    ) -> syn::Result<Self> {
        let state_param = item_struct.state_param(args.state.as_ref())?;
        state_param
            .bounds
            .push(parse_quote!(::typestate_groups::State));

        let target_state = TypeParam {
            ident: crate::naming::target_state_ident(),
            ..state_param.clone()
        };
        let state = state_param.ident.clone();
        let target_ty =
            item_struct.with_state(&state, &target_state.ident);
        item_struct.require_no_marker(&state)?;

        let shapes = match &args.transmute {
            Transmute::On(align) => {
                let shapes = item_struct
                    .fields
                    .iter()
                    .map(|field| field.shape(&state))
                    .collect::<syn::Result<Vec<_>>>()?;
                if shapes
                    .iter()
                    .all(|shape| matches!(shape, FieldShape::Fixed))
                {
                    return Err(syn::Error::new_spanned(
                        &item_struct.ident,
                        format!(
                            "add a field of type `{state}::Assoc` or a \
                             pointer to one, or remove `unsafe_transmute \
                             = true`",
                        ),
                    ));
                }
                item_struct.ensure_repr(align)?;
                shapes
            }
            Transmute::Off => Vec::new(),
        };

        Ok(TypeState {
            item_struct,
            state,
            target_state,
            target_ty,
            transmute: args.transmute,
            shapes,
        })
    }

    /// The struct followed by every impl `#[typestate]` derives for it.
    pub(crate) fn generate_typestate_impls(&self) -> TokenStream {
        let item_struct = &self.item_struct;
        let with_state_impl = self.with_state_impl();
        let restate_impl = self.restate_impl();
        let transmute_impls = match &self.transmute {
            Transmute::On(align) => {
                let transmutable_state_impl =
                    self.transmutable_state_impl(align);
                let castable_state_impls = self.castable_state_impls();
                Some(quote! {
                    #transmutable_state_impl

                    #castable_state_impls
                })
            }
            Transmute::Off => None,
        };

        quote! {
            #item_struct

            #with_state_impl

            #restate_impl

            #transmute_impls
        }
    }

    fn with_state_impl(&self) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let state = &self.state;
        let (impl_generics, ty_generics, where_clause) =
            self.item_struct.generics.split_for_impl();

        quote! {
            impl #impl_generics ::typestate_groups::WithState for #struct_ident #ty_generics #where_clause {
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
            impl #impl_generics ::typestate_groups::Restate<#target_state>
                for #struct_ident #ty_generics #where_clause
            {
                type Target = #target_ty;
            }
        }
    }

    /// The struct's generics plus the target state, which gets a copy of
    /// every `where` predicate on the state and of every outlives bound
    /// a reference field implies.
    ///
    /// `where S: Debug` -> `where S: Debug, S2: Debug`,
    /// `field: &'a S::Value` -> `where S2::Value: 'a`
    fn target_generics(&self) -> Generics {
        let mut generics = self.item_struct.generics.clone();
        let implied = self
            .item_struct
            .fields
            .iter()
            .flat_map(|field| field.ty.outlives(&self.state));
        let target_predicates: Vec<WherePredicate> = generics
            .where_clause
            .iter()
            .flat_map(|clause| clause.predicates.iter().cloned())
            .chain(implied)
            .filter_map(|predicate| {
                predicate.renamed(&self.state, &self.target_state.ident)
            })
            .collect();

        generics
            .params
            .push(GenericParam::Type(self.target_state.clone()));
        generics
            .make_where_clause()
            .predicates
            .extend(target_predicates);
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
            self.projections().map(|projection| {
                self.layout_predicate(projection, align)
            }),
        );
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let layout_check = self.layout_check();

        quote! {
            // SAFETY: the where-clause gives every projection one size
            // in both states, a pointer's layout ignores its sized
            // pointee, and the `repr` fixes the field order.
            // `LAYOUT_CHECK` rejects any alignment or field offset that
            // `align = N` lets differ, and a pointer that turns fat.
            unsafe impl #impl_generics ::typestate_groups::TransmutableState<#target_state>
                for #struct_ident #ty_generics #where_clause
            {
                #layout_check
            }
        }
    }

    /// `CastableState<S2, A>` for `Struct<S>`, for every access `A` and
    /// every `S2` whose projections and pointees stay valid under it.
    fn castable_state_impls(&self) -> TokenStream {
        let struct_ident = &self.item_struct.ident;
        let target_state = &self.target_state.ident;
        let (_, ty_generics, _) =
            self.item_struct.generics.split_for_impl();
        let pointee_check = self.pointee_check();

        Access::ALL
            .into_iter()
            .map(|access| {
                let mut generics = self.target_generics();
                let predicates = &mut generics.make_where_clause().predicates;
                predicates.push(parse_quote! {
                    #struct_ident #ty_generics:
                        ::typestate_groups::TransmutableState<#target_state>
                });
                predicates.extend(
                    self.shapes
                        .iter()
                        .flat_map(|shape| self.cast_predicates(shape, access)),
                );
                let (impl_generics, _, where_clause) =
                    generics.split_for_impl();
                let access = access.path();

                quote! {
                    // SAFETY: the where-clause proves every projection and
                    // pointee valid in the target state under this access,
                    // `POINTEE_CHECK` keeps every pointee's size and
                    // alignment, and every other field keeps its type.
                    unsafe impl #impl_generics ::typestate_groups::CastableState<#target_state, #access>
                        for #struct_ident #ty_generics #where_clause
                    {
                        #pointee_check
                    }
                }
            })
            .collect()
    }

    /// The bounds that keep a field valid in the target state when the
    /// container is cast through `access`.
    ///
    /// `S::P` follows `access`. A pointee follows its pointer's
    /// [`PointerKind::accesses`].
    fn cast_predicates(
        &self,
        shape: &FieldShape,
        access: Access,
    ) -> Vec<WherePredicate> {
        let state = &self.state;
        let target_state = &self.target_state.ident;

        match shape {
            FieldShape::Fixed => Vec::new(),
            FieldShape::Projection(assoc) => access.predicates(
                &parse_quote!(#state::#assoc),
                &parse_quote!(#target_state::#assoc),
            ),
            FieldShape::Pointer { kind, pointee } => {
                let target = pointee.renamed(state, target_state);
                kind.accesses(access)
                    .into_iter()
                    .flat_map(|access| access.predicates(pointee, &target))
                    .collect()
            }
        }
    }

    /// `CastableState::POINTEE_CHECK`, with one size and alignment
    /// assertion per pointer field.
    fn pointee_check(&self) -> TokenStream {
        let state = &self.state;
        let target_state = &self.target_state.ident;
        let asserts = self
            .item_struct
            .fields
            .members()
            .zip(&self.shapes)
            .filter_map(|(member, shape)| match shape {
                FieldShape::Pointer { pointee, .. } => {
                    let target = pointee.renamed(state, target_state);
                    let msg = format!(
                        "make the pointee of `{}` keep its size and \
                         alignment in the target state, or convert with \
                         `morph`",
                        quote!(#member),
                    );
                    Some(quote! {
                        ::core::assert!(
                            ::core::mem::size_of::<#pointee>()
                                == ::core::mem::size_of::<#target>()
                                && ::core::mem::align_of::<#pointee>()
                                    == ::core::mem::align_of::<#target>(),
                            #msg
                        );
                    })
                }
                FieldShape::Fixed | FieldShape::Projection(_) => None,
            });

        quote! {
            const POINTEE_CHECK: () = {
                #(#asserts)*
            };
        }
    }

    /// The associated types the fields hold by value (`Value` in a field
    /// of type `S::Value`).
    fn projections(&self) -> impl Iterator<Item = &Ident> {
        self.shapes.iter().filter_map(|shape| match shape {
            FieldShape::Projection(assoc) => Some(assoc),
            FieldShape::Fixed | FieldShape::Pointer { .. } => None,
        })
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

    /// `S2::__TypestateGroupsLayoutP:
    /// SameLayout<S::__TypestateGroupsLayoutP>`, or only `SameSize`
    /// under `align = N`.
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
                #target_state::#layout: ::typestate_groups::SameLayout<#state::#layout>
            },
            Alignment::Forced(_) => parse_quote! {
                <#target_state::#layout as ::typestate_groups::TypeLayout>::Size:
                    ::typestate_groups::SameSize<<#state::#layout as ::typestate_groups::TypeLayout>::Size>
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

/// How a field changes between states, for transmuting.
#[expect(
    clippy::large_enum_variant,
    reason = "one per field, built once per macro call"
)]
pub(crate) enum FieldShape {
    /// The same type in every state: no `S`, or a ZST such as
    /// `PhantomData<S::Value>`.
    Fixed,
    /// `S::Assoc`, laid out by its group's `#[size(N)]`.
    Projection(Ident),
    /// A pointer whose layout doesn't depend on its sized pointee.
    Pointer { kind: PointerKind, pointee: Type },
}

/// A pointer field's kind, which decides who else can see its pointee.
#[derive(Clone, Copy)]
pub(crate) enum PointerKind {
    /// `*const T`, `*mut T` or `NonNull<T>`.
    Raw,
    /// `&T`
    Shared,
    /// `&mut T`
    Unique,
    /// `Box<T>`
    Boxed,
}

impl PointerKind {
    /// The accesses whose bounds the pointee needs when the container is
    /// cast through `container`.
    ///
    /// A `&T` pointee may be aliased, so it needs `Shared` even by value.
    /// A `&mut T` pointee goes back to its lender, so it needs
    /// `Exclusive`. A raw pointee may be either.
    fn accesses(self, container: Access) -> Vec<Access> {
        match self {
            PointerKind::Boxed => vec![container],
            PointerKind::Shared => vec![Access::Shared],
            PointerKind::Unique => match container {
                Access::Shared => vec![Access::Shared],
                Access::Owned | Access::Exclusive => {
                    vec![Access::Exclusive]
                }
            },
            PointerKind::Raw => vec![Access::Shared, Access::Exclusive],
        }
    }
}

/// How a cast holds a value, as `typestate_groups::Access`.
#[derive(Clone, Copy)]
pub(crate) enum Access {
    Owned,
    Shared,
    Exclusive,
}

impl Access {
    const ALL: [Access; 3] =
        [Access::Owned, Access::Shared, Access::Exclusive];

    /// `::typestate_groups::Owned`
    fn path(self) -> TokenStream {
        match self {
            Access::Owned => quote!(::typestate_groups::Owned),
            Access::Shared => quote!(::typestate_groups::Shared),
            Access::Exclusive => quote!(::typestate_groups::Exclusive),
        }
    }

    /// The bounds that keep a `src` valid as a `dst` under this access.
    ///
    /// `Owned` needs `dst: CastFrom<src>`, `Shared` adds
    /// `dst: CastRefFrom<src>` and `Exclusive` adds `src: CastFrom<dst>`.
    fn predicates(self, src: &Type, dst: &Type) -> Vec<WherePredicate> {
        let forward =
            parse_quote!(#dst: ::typestate_groups::CastFrom<#src>);
        match self {
            Access::Owned => vec![forward],
            Access::Shared => vec![
                forward,
                parse_quote!(#dst: ::typestate_groups::CastRefFrom<#src>),
            ],
            Access::Exclusive => vec![
                forward,
                parse_quote!(#src: ::typestate_groups::CastFrom<#dst>),
            ],
        }
    }
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

    /// Rejects a field that mentions `S::Marker` at any depth.
    fn require_no_marker(&self, state: &Ident) -> syn::Result<()> {
        let marker = self
            .fields
            .iter()
            .flat_map(|field| field.ty.projections(state))
            .find(|projection| projection == "Marker");

        match marker {
            Some(marker) => Err(syn::Error::new(
                marker.span(),
                format!(
                    "replace `{state}::Marker` with \
                     `PhantomData<{state}>`: a group marker has no value \
                     to convert"
                ),
            )),
            None => Ok(()),
        }
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
    /// How this field changes between states, or an error when its layout
    /// may change in a way `#[typestate]` can't check.
    ///
    /// `S::Value -> Projection(Value)`,
    /// `Option<NonNull<S::Value>> -> Pointer { Raw, S::Value }`
    fn shape(&self, state: &Ident) -> syn::Result<FieldShape> {
        let ty = &self.ty;
        if !ty.mentions_ident(state) || ty.is_zst() {
            return Ok(FieldShape::Fixed);
        }
        if let Some(assoc) = ty.state_projection(state) {
            return Ok(FieldShape::Projection(assoc.clone()));
        }
        if let Some((kind, pointee)) = ty.pointer() {
            return Ok(FieldShape::Pointer {
                kind,
                pointee: pointee.clone(),
            });
        }

        Err(syn::Error::new_spanned(
            ty,
            format!(
                "make this field `{state}::Assoc`, a pointer to a sized \
                 type (`*const`, `*mut`, `NonNull`, `&`, `&mut` or \
                 `Box`, or one of the last four in an `Option`), a ZST \
                 or a type without `{state}`, or remove \
                 `unsafe_transmute = true` and convert with `morph`"
            ),
        ))
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

    /// The pointer kind and pointee when this type is a pointer whose
    /// layout doesn't depend on a sized pointee. `Option` only wraps the
    /// pointers std guarantees a null niche for.
    ///
    /// `Option<NonNull<S::Value>> -> (Raw, S::Value)`
    fn pointer(&self) -> Option<(PointerKind, &Type)> {
        match self.peeled() {
            Type::Ptr(ptr) => Some((PointerKind::Raw, &ptr.elem)),
            Type::Reference(TypeReference {
                mutability, elem, ..
            }) => match mutability {
                Some(_) => Some((PointerKind::Unique, elem)),
                None => Some((PointerKind::Shared, elem)),
            },
            Type::Path(TypePath {
                qself: None, path, ..
            }) => {
                let arg = path.single_type_arg()?;
                if path.is_std_item("ptr", "NonNull") {
                    Some((PointerKind::Raw, arg))
                } else if path.is_std_item("boxed", "Box") {
                    Some((PointerKind::Boxed, arg))
                } else if path.is_std_item("option", "Option")
                    && !matches!(arg.peeled(), Type::Ptr(_))
                {
                    arg.pointer()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// `T: 'a` for every `&'a T` inside this type whose `T` mentions
    /// `state`.
    ///
    /// `Option<&'a S::Value>` -> `[S::Value: 'a]`
    fn outlives(&self, state: &Ident) -> Vec<WherePredicate> {
        let mut outlives = Outlives {
            state,
            found: Vec::new(),
        };
        outlives.visit_type(self);
        outlives.found
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

/// Collects `T: 'a` for every `&'a T` it visits whose `T` mentions the
/// state.
struct Outlives<'a> {
    state: &'a Ident,
    found: Vec<WherePredicate>,
}

impl<'ast> Visit<'ast> for Outlives<'_> {
    fn visit_type_reference(&mut self, reference: &'ast TypeReference) {
        if let TypeReference {
            lifetime: Some(lifetime),
            elem,
            ..
        } = reference
            && elem.mentions_ident(self.state)
        {
            self.found.push(parse_quote!(#elem: #lifetime));
        }
        visit::visit_type_reference(self, reference);
    }
}
