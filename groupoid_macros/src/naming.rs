use quote::format_ident;
use syn::Ident;

/// The marker trait `#[template]` generates, e.g. `A` -> `AGroupMarker`.
pub fn group_marker_ident(trait_ident: &Ident) -> Ident {
    format_ident!("{}GroupMarker", trait_ident)
}

/// The helper trait's own identifier, e.g. `A` -> `AHelper`.
pub fn helper_trait_ident(trait_ident: &Ident) -> Ident {
    format_ident!("{}Helper", trait_ident)
}

/// The module `#[group_trait]` generates to hold the helper trait, e.g.
/// `A` -> `__a_helper_mod`.
pub fn helper_mod_ident(trait_ident: &Ident) -> Ident {
    format_ident!(
        "__{}_helper_mod",
        trait_ident.to_string().to_lowercase()
    )
}

/// The state parameter `morph_with` transitions to.
pub fn target_state_ident() -> Ident {
    format_ident!("__GroupoidTargetState")
}

/// The conversion closure parameter of `morph_with`.
pub fn morph_fn_ident() -> Ident {
    format_ident!("f")
}

/// The binding for one projected value inside `morph_with`.
pub fn morph_elem_ident() -> Ident {
    format_ident!("v")
}

/// The binding for element `index` of a destructured tuple, e.g. `v0`.
pub fn tuple_binding_ident(index: usize) -> Ident {
    format_ident!("v{index}")
}
