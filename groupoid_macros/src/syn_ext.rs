//! Extension methods on `syn` and `proc_macro2` types that know nothing
//! about groupoid itself: syntax queries and token rewrites any macro
//! could use.

use extend::ext;
use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Attribute, Generics, Ident, Path, Token, Type, TypeParam, TypePath,
    parse::{Parse, ParseStream},
};

#[ext]
pub(crate) impl Type {
    /// This type with its parentheses, and the invisible groups a
    /// `macro_rules!` `$ty` substitution wraps it in, stripped off.
    fn peeled(&self) -> &Type {
        match self {
            Type::Paren(paren) => paren.elem.peeled(),
            Type::Group(group) => group.elem.peeled(),
            ty => ty,
        }
    }

    /// A fresh value of this type, when its spelling shows it to be a ZST:
    /// `()` or `PhantomData<..>`.
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

    /// This type's tokens with every `from` that heads a path replaced by
    /// `to`, e.g. `Pair<S::Value>` ->
    /// `Pair<__GroupoidTargetState::Value>`.
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
    /// Whether this path names `item` from std's `module`, whether spelled
    /// bare (`Option`), through the module (`option::Option`) or through a
    /// root crate (`core::option::Option`).
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
}

#[ext]
pub(crate) impl Generics {
    /// The type parameter named `ident`, if these generics declare one.
    fn type_param_mut(&mut self, ident: &Ident) -> Option<&mut TypeParam> {
        self.type_params_mut().find(|tp| tp.ident == *ident)
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
    /// Parses `= <value>` for the argument `key` into this slot, rejecting
    /// a second assignment.
    fn parse_once(
        &mut self,
        key: &Ident,
        input: ParseStream,
    ) -> syn::Result<()> {
        if self.is_some() {
            return Err(syn::Error::new(
                key.span(),
                format!("duplicate `{key}` argument"),
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
    /// These tokens with every `from` not preceded by `:` replaced by
    /// `to`.
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
