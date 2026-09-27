use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemTrait, TraitItem, TraitItemType, parse_quote};

pub struct Template<'ast> {
    inner: &'ast ItemTrait,
    /// The trait's associated types, one or more.
    types: Vec<&'ast TraitItemType>,
}

impl<'ast> Template<'ast> {
    pub fn new(inner: &'ast ItemTrait) -> syn::Result<Template<'ast>> {
        let types: Vec<&TraitItemType> = inner
            .items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Type(ty) => Some(ty),
                _ => None,
            })
            .collect();

        if types.is_empty() {
            return Err(syn::Error::new_spanned(
                &inner.ident,
                format!(
                    "add an associated type to `{}`, such as `type \
                     Value;`, for its groups to set",
                    inner.ident
                ),
            ));
        }

        Ok(Template { inner, types })
    }

    /// The trait with a `Marker` associated type, a hidden
    /// `__GroupoidLayout{Assoc}` associated type per associated type that
    /// `#[group]` sets, and `State` and hidden keys supertraits, followed
    /// by its `{Trait}GroupMarker` trait, its `{Trait}GroupMember<G>`
    /// trait, which every state of group `G` implements, its
    /// `{Trait}Morph` trait and its keys.
    ///
    /// ```ignore
    /// trait Meta { type A; type B; }
    /// // ->
    /// trait Meta: ::groupoid::State + __MetaKeys {
    ///     type A;
    ///     type B;
    ///     type __GroupoidLayoutA;
    ///     type __GroupoidLayoutB;
    ///     type Marker: MetaGroupMarker<A = Self::A, B = Self::B>;
    /// }
    /// trait MetaGroupMarker { type A; type B; }
    /// trait MetaGroupMember<G: MetaGroupMarker>:
    ///     Meta<Marker = G, A = G::A, B = G::B> {}
    /// trait MetaMorph<Src: Meta, Dst: Meta> {
    ///     fn a(&mut self, value: Src::A) -> Dst::A;
    ///     fn b(&mut self, value: Src::B) -> Dst::B;
    /// }
    /// ```
    ///
    /// See [`Template::keys`] for the keys.
    pub fn create_group_marker(&self) -> syn::Result<TokenStream> {
        let vis = &self.inner.vis;
        let trait_ident = &self.inner.ident;
        let marker_name = crate::naming::group_marker_ident(trait_ident);
        let member_name = crate::naming::group_member_ident(trait_ident);
        let group = crate::naming::group_param_ident();
        let types = &self.types;
        let assocs: Vec<_> = types.iter().map(|ty| &ty.ident).collect();

        let mut original = self.inner.clone();
        original
            .items
            .extend(assocs.iter().map(|assoc| -> TraitItem {
                let layout = crate::naming::layout_assoc_ident(assoc);
                parse_quote! {
                    #[doc(hidden)]
                    type #layout;
                }
            }));
        original.items.push(parse_quote! {
            type Marker: #marker_name<#(#assocs = Self::#assocs),*>;
        });
        original.supertraits.push(parse_quote!(::groupoid::State));
        let keys_trait = crate::naming::keys_trait_ident(trait_ident);
        original.supertraits.push(parse_quote!(#keys_trait));

        let member_bound = quote! {
            #trait_ident<Marker = #group, #(#assocs = #group::#assocs),*>
        };

        let morph_trait = self.morph_trait();
        let keys = self.keys();

        Ok(quote! {
            #original

            #vis trait #marker_name {
                #(#types)*
            }

            #vis trait #member_name<#group: #marker_name>: #member_bound {}

            impl<#group: #marker_name, T: #member_bound> #member_name<#group> for T {}

            #morph_trait

            #keys
        })
    }

    /// `{Trait}Morph<Src, Dst>`, one method per associated type, and its
    /// impl for `&mut M`, so one morpher can convert several values.
    fn morph_trait(&self) -> TokenStream {
        let vis = &self.inner.vis;
        let trait_ident = &self.inner.ident;
        let morph_trait = crate::naming::morph_trait_ident(trait_ident);
        let assocs: Vec<_> =
            self.types.iter().map(|ty| &ty.ident).collect();
        let methods: Vec<_> = assocs
            .iter()
            .map(|assoc| crate::naming::morph_method_ident(assoc))
            .collect();
        let docs = assocs.iter().map(|assoc| {
            format!("Converts `Src::{assoc}` into `Dst::{assoc}`.")
        });
        let trait_doc = format!(
            "Converts every associated type of [`{trait_ident}`] from \
             state `Src` to `Dst`."
        );
        let message = format!(
            "implement `{morph_trait}<{{Src}}, {{Dst}}>` for `{{Self}}` \
             to morph from `{{Src}}` to `{{Dst}}`"
        );

        quote! {
            #[doc = #trait_doc]
            ///
            /// Implement it on any type, then pass that type to `morph` on
            /// a `#[typestate]` struct. `&mut` an implementor implements it
            /// too, so one morpher can convert several values.
            #[diagnostic::on_unimplemented(message = #message)]
            #vis trait #morph_trait<Src: #trait_ident, Dst: #trait_ident> {
                #(
                    #[doc = #docs]
                    fn #methods(&mut self, value: Src::#assocs) -> Dst::#assocs;
                )*
            }

            impl<M, Src, Dst> #morph_trait<Src, Dst> for &mut M
            where
                M: #morph_trait<Src, Dst> + ?Sized,
                Src: #trait_ident,
                Dst: #trait_ident,
            {
                #(
                    fn #methods(&mut self, value: Src::#assocs) -> Dst::#assocs {
                        (**self).#methods(value)
                    }
                )*
            }
        }
    }

    /// One key type per associated type, the hidden supertrait naming them
    /// from any state, and the `MorphLeaf` bridge from each key to its
    /// `{Trait}Morph` method.
    ///
    /// `#[typestate]` can't name the template, but it can name
    /// `S::__GroupoidKeyA` through the state, and `morph` converts `S::A`
    /// through `<S::__GroupoidKeyA as MorphLeaf<M, S, S2>>::morph_leaf`.
    ///
    /// ```ignore
    /// struct __MetaKeyA;
    /// trait __MetaKeys { type __GroupoidKeyA; }
    /// impl<T: ?Sized> __MetaKeys for T { type __GroupoidKeyA = __MetaKeyA; }
    /// impl<M: MetaMorph<Src, Dst>, Src: Meta, Dst: Meta>
    ///     MorphLeaf<M, Src, Dst> for __MetaKeyA { .. m.a(value) .. }
    /// ```
    ///
    /// Each key has the template's visibility, so its `MorphLeaf` impl
    /// is no more public than the associated types it names.
    fn keys(&self) -> TokenStream {
        let vis = &self.inner.vis;
        let trait_ident = &self.inner.ident;
        let morph_trait = crate::naming::morph_trait_ident(trait_ident);
        let keys_trait = crate::naming::keys_trait_ident(trait_ident);
        let assocs: Vec<_> =
            self.types.iter().map(|ty| &ty.ident).collect();
        let key_assocs: Vec<_> = assocs
            .iter()
            .map(|assoc| crate::naming::key_assoc_ident(assoc))
            .collect();
        let keys: Vec<_> = assocs
            .iter()
            .map(|assoc| {
                crate::naming::key_struct_ident(trait_ident, assoc)
            })
            .collect();
        let methods = assocs
            .iter()
            .map(|assoc| crate::naming::morph_method_ident(assoc));

        quote! {
            #(
                #[doc(hidden)]
                #vis struct #keys;
            )*

            #[doc(hidden)]
            #vis trait #keys_trait {
                #(type #key_assocs;)*
            }

            impl<T: ?Sized> #keys_trait for T {
                #(type #key_assocs = #keys;)*
            }

            #(
                impl<M, Src, Dst> ::groupoid::MorphLeaf<M, Src, Dst> for #keys
                where
                    M: #morph_trait<Src, Dst>,
                    Src: #trait_ident,
                    Dst: #trait_ident,
                {
                    type In = Src::#assocs;
                    type Out = Dst::#assocs;

                    fn morph_leaf(m: &mut M, value: Src::#assocs) -> Dst::#assocs {
                        m.#methods(value)
                    }
                }
            )*
        }
    }
}
