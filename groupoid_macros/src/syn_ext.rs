//! Project-agnostic extensions on `syn` and `proc_macro2` types.

use extend::ext;
use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    AssocType, Attribute, Generics, Ident, Path, PredicateType, Token,
    Type, TypeParam, TypeParamBound, TypePath, WherePredicate,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    visit::Visit,
};

#[ext]
pub(crate) impl Type {
    /// This type without parentheses and invisible groups.
    fn peeled(&self) -> &Type {
        match self {
            Type::Paren(paren) => paren.elem.peeled(),
            Type::Group(group) => group.elem.peeled(),
            ty => ty,
        }
    }

    /// A value of this type when it's `()` or `PhantomData<..>`.
    fn zst_value(&self) -> Option<TokenStream> {
        let path = match self.peeled() {
            Type::Tuple(tuple) => {
                return tuple.elems.is_empty().then(|| quote!(()));
            }
            Type::Path(TypePath {
                qself: None, path, ..
            }) => path,
            _ => return None,
        };

        if path.is_std_item("marker", "PhantomData") {
            Some(quote!(::core::marker::PhantomData))
        } else {
            None
        }
    }

    /// Whether this type's spelling shows it to be a ZST.
    fn is_zst(&self) -> bool {
        self.zst_value().is_some()
    }

    /// `Pair<S::Value>` -> `Pair<To::Value>`
    fn with_ident_renamed(&self, from: &Ident, to: &Ident) -> TokenStream {
        self.to_token_stream().rename_ident(from, to)
    }

    /// Whether `ident` appears anywhere inside this type.
    fn mentions_ident(&self, ident: &Ident) -> bool {
        self.to_token_stream().mentions_ident(ident)
    }
}

#[ext]
pub(crate) impl Path {
    /// Whether this path names std's `module::item`, with any prefix.
    fn is_std_item(&self, module: &str, item: &str) -> bool {
        let idents: Vec<&Ident> =
            self.segments.iter().map(|s| &s.ident).collect();

        let Some((last, prefix)) = idents.split_last() else {
            return false;
        };
        *last == item
            && match prefix {
                [] => true,
                [m] => *m == module,
                [root, m] => {
                    (*root == "std" || *root == "core" || *root == "alloc")
                        && *m == module
                }
                _ => false,
            }
    }

    /// `a::Meta` -> `a::MetaGroupMarker`
    fn with_last_ident(
        &self,
        rename: impl FnOnce(&Ident) -> Ident,
    ) -> Path {
        let mut path = self.clone();
        if let Some(last) = path.segments.last_mut() {
            last.ident = rename(&last.ident);
        }
        path
    }
}

#[ext]
pub(crate) impl Generics {
    /// The type parameter named `ident`, if these generics declare one.
    fn type_param_mut(&mut self, ident: &Ident) -> Option<&mut TypeParam> {
        self.type_params_mut().find(|tp| tp.ident == *ident)
    }

    /// `S: Meta<Value = String>` -> `Value = String`
    fn assoc_type_binding(&self, param: &Ident) -> Option<&AssocType> {
        #[derive(Default)]
        struct Finder<'ast>(Option<&'ast AssocType>);

        impl<'ast> Visit<'ast> for Finder<'ast> {
            fn visit_assoc_type(&mut self, assoc: &'ast AssocType) {
                self.0.get_or_insert(assoc);
            }
        }

        let inline = self
            .type_params()
            .filter(|tp| tp.ident == *param)
            .flat_map(|tp| &tp.bounds);
        let in_where = self
            .where_clause
            .iter()
            .flat_map(|clause| &clause.predicates)
            .filter_map(|predicate| predicate.bounds_on(param))
            .flatten();

        inline.chain(in_where).find_map(|bound| {
            let mut finder = Finder::default();
            finder.visit_type_param_bound(bound);
            finder.0
        })
    }
}

#[ext]
pub(crate) impl WherePredicate {
    /// This predicate's bounds when it bounds `param`.
    ///
    /// `S: Meta + Clone` -> `Meta + Clone`
    fn bounds_on(
        &self,
        param: &Ident,
    ) -> Option<&Punctuated<TypeParamBound, Token![+]>> {
        match self {
            WherePredicate::Type(PredicateType {
                bounded_ty:
                    Type::Path(TypePath {
                        qself: None, path, ..
                    }),
                bounds,
                ..
            }) if path.is_ident(param) => Some(bounds),
            _ => None,
        }
    }
}

#[ext]
pub(crate) impl Attribute {
    /// Whether this attribute is `#[repr(C | transparent)]`.
    fn guarantees_layout(&self) -> bool {
        self.meta.require_list().is_ok_and(|list| {
            list.tokens
                .clone()
                .into_iter()
                .any(|tt| matches!(&tt, TokenTree::Ident(i) if i == "C" || i == "transparent"))
        })
    }
}

#[ext(name = OptionExt)]
pub(crate) impl<T: Parse> Option<T> {
    /// Parses `= <value>` into this slot, rejecting duplicates.
    fn parse_once(
        &mut self,
        key: &Ident,
        input: ParseStream,
    ) -> syn::Result<()> {
        if self.is_some() {
            return Err(syn::Error::new(
                key.span(),
                format!("remove the duplicate `{key}` argument"),
            ));
        }
        input.parse::<Token![=]>()?;
        *self = Some(input.parse()?);
        Ok(())
    }
}

/// Token scans behind the `Type` methods of the same names.
#[ext]
impl TokenStream {
    /// These tokens with every `from` not after `:` renamed to `to`.
    fn rename_ident(self, from: &Ident, to: &Ident) -> TokenStream {
        let mut after_colon = false;
        self.into_iter()
            .map(|tt| {
                let renamed = match tt {
                    TokenTree::Ident(ref ident)
                        if ident == from && !after_colon =>
                    {
                        TokenTree::Ident(to.clone())
                    }
                    TokenTree::Group(group) => {
                        let mut renamed = proc_macro2::Group::new(
                            group.delimiter(),
                            group.stream().rename_ident(from, to),
                        );
                        renamed.set_span(group.span());
                        TokenTree::Group(renamed)
                    }
                    tt => tt,
                };
                after_colon = renamed.is_colon();
                renamed
            })
            .collect()
    }

    /// Whether `ident` appears anywhere in these tokens.
    fn mentions_ident(self, ident: &Ident) -> bool {
        self.into_iter().any(|tt| match tt {
            TokenTree::Ident(i) => i == *ident,
            TokenTree::Group(g) => g.stream().mentions_ident(ident),
            _ => false,
        })
    }
}

#[ext]
impl TokenTree {
    /// Whether this token is one `:`.
    fn is_colon(&self) -> bool {
        matches!(self, TokenTree::Punct(p) if p.as_char() == ':')
    }
}
