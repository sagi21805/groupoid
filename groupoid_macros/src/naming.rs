use quote::format_ident;
use syn::Ident;

/// The helper trait's own identifier, e.g. `A` -> `AHelper`.
pub fn helper_trait_ident(trait_ident: &Ident) -> Ident {
    format_ident!("{}Helper", trait_ident)
}

/// The identifier of the module that `#[group_trait]` generates to hold the
/// helper trait, keeping it out of the surrounding scope so its methods
/// don't create ambiguity with the public trait's methods of the same name.
/// `#[group_impl]` must derive the same identifier to reach the helper trait
/// it implements, so both macros go through this one function.
pub fn helper_mod_ident(trait_ident: &Ident) -> Ident {
    format_ident!("__{}_helper_mod", trait_ident.to_string().to_lowercase())
}
