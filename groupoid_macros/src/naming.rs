use proc_macro2::Span;
use quote::format_ident;
use syn::{Ident, Lifetime};

/// `A` -> `AGroupMarker`
pub fn group_marker_ident(trait_ident: &Ident) -> Ident {
    format_ident!("{}GroupMarker", trait_ident)
}

/// `A` -> `AGroupMember`
pub fn group_member_ident(trait_ident: &Ident) -> Ident {
    format_ident!("{}GroupMember", trait_ident)
}

/// `Value` -> `__GroupoidLayoutValue`
pub fn layout_assoc_ident(assoc: &Ident) -> Ident {
    format_ident!("__GroupoidLayout{}", assoc)
}

/// `Meta` -> `MetaMorph`
pub fn morph_trait_ident(trait_ident: &Ident) -> Ident {
    format_ident!("{}Morph", trait_ident)
}

/// `Meta`, `Value` -> `__MetaKeyValue`
pub fn key_struct_ident(trait_ident: &Ident, assoc: &Ident) -> Ident {
    format_ident!("__{}Key{}", trait_ident, assoc)
}

/// `Meta` -> `__MetaKeys`
pub fn keys_trait_ident(trait_ident: &Ident) -> Ident {
    format_ident!("__{}Keys", trait_ident)
}

/// `Value` -> `__GroupoidKeyValue`
pub fn key_assoc_ident(assoc: &Ident) -> Ident {
    format_ident!("__GroupoidKey{}", assoc)
}

/// The group parameter of `{Trait}GroupMember`.
pub fn group_param_ident() -> Ident {
    format_ident!("__GroupoidGroup")
}

/// `{Trait}GroupMember`'s alias in a `#[group_trait]` helper module.
pub fn helper_member_ident() -> Ident {
    format_ident!("Member")
}

/// `A` -> `AHelper`
pub fn helper_trait_ident(trait_ident: &Ident) -> Ident {
    format_ident!("{}Helper", trait_ident)
}

/// `A` -> `__a_helper_mod`
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

/// The struct `#[typestate]` generates to hold one conversion per
/// projection, e.g. `Wrap` -> `WrapMorph`.
pub fn struct_morph_ident(struct_ident: &Ident) -> Ident {
    format_ident!("{}Morph", struct_ident)
}

/// The lifetime of the conversions in a `{Struct}Morph`.
pub fn morph_lifetime() -> Lifetime {
    Lifetime::new("'f", Span::call_site())
}

/// The `{Struct}Morph` field and `{Template}Morph` method converting
/// associated type `assoc`, in snake case, e.g. `MyValue` -> `my_value`
/// and `Type` -> `r#type`.
pub fn morph_method_ident(assoc: &Ident) -> Ident {
    let snake = snake_case(&assoc.to_string());

    match snake.as_str() {
        "self" | "super" | "crate" => format_ident!("{snake}_"),
        keyword if KEYWORDS.contains(&keyword) => {
            Ident::new_raw(keyword, assoc.span())
        }
        _ => Ident::new(&snake, assoc.span()),
    }
}

/// The binding for `assoc`'s conversion inside `morph_with`, e.g.
/// `Value` -> `__groupoid_value`.
pub fn morph_binding_ident(assoc: &Ident) -> Ident {
    format_ident!("__groupoid_{}", snake_case(&assoc.to_string()))
}

/// The morpher type parameter of `morph`.
pub fn morpher_param_ident() -> Ident {
    format_ident!("__GroupoidMorpher")
}

/// The morpher parameter of `morph`.
pub fn morpher_ident() -> Ident {
    format_ident!("m")
}

/// The conversion closure parameter of `morph_with`.
pub fn morph_fn_ident() -> Ident {
    format_ident!("f")
}

/// The binding for one projected value inside `morph_with`.
pub fn morph_elem_ident() -> Ident {
    format_ident!("v")
}

/// `0` -> `v0`
pub fn tuple_binding_ident(index: usize) -> Ident {
    format_ident!("v{index}")
}

/// Rust's strict and reserved keywords that can be raw identifiers.
const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const",
    "continue", "do", "dyn", "else", "enum", "extern", "false", "final",
    "fn", "for", "gen", "if", "impl", "in", "let", "loop", "macro",
    "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "static", "struct", "trait", "true", "try", "type",
    "typeof", "unsafe", "unsized", "use", "virtual", "where", "while",
    "yield",
];

/// `MyValue` -> `my_value`, `HTTPCode` -> `http_code`, `Type1` ->
/// `type1`.
fn snake_case(camel: &str) -> String {
    let chars: Vec<char> = camel.chars().collect();
    let mut snake = String::with_capacity(camel.len() + 4);

    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() && i > 0 {
            let prev = chars[i - 1];
            let next_is_lower =
                chars.get(i + 1).is_some_and(|next| next.is_lowercase());
            if prev.is_lowercase()
                || prev.is_ascii_digit()
                || (prev.is_uppercase() && next_is_lower)
            {
                snake.push('_');
            }
        }
        snake.extend(c.to_lowercase());
    }

    snake
}
