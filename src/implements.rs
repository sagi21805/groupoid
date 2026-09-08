use syn::{
    Token, TypePath,
    parse::{Parse, discouraged::Speculative},
    parse_quote,
};

mod kw {
    syn::custom_keyword!(state);
}

/// Implements a [`group_trait`] in a generic way for the tagged struct.
///
/// # Usage
/// ```rust
/// #[implements(TraitA, TraitB, state = S)]
/// struct Example<S: StateTrait> {}
/// ```
pub struct Implements {
    implemented_traits: Vec<TypePath>,
    state_generic: TypePath,
}

impl Parse for Implements {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut seen_state: Option<proc_macro2::Span> = None;
        let mut implemented_traits = Vec::new();
        let mut state_generic = parse_quote!();

        while !input.is_empty() {
            let mut error_count = 0;

            let state = try_parse::<GenericState>(&input, Some(&mut seen_state), &mut error_count)
                .transpose()?;

            if let Some(generic) = state {
                state_generic = generic.ty;
            }

            let implemented_trait =
                try_parse::<TypePath>(&input, None, &mut error_count).transpose()?;

            if let Some(implemented) = implemented_trait {
                implemented_traits.push(implemented);
            }

            // Couldn't parse any part of the attribute.
            if error_count >= 2 {
                let unknown: proc_macro2::TokenTree = input.parse()?;
                return Err(syn::Error::new_spanned(
                    &unknown,
                    format!("unknown option: {}", unknown),
                ));
            }
        }

        if seen_state.is_none() {
            let unknown: proc_macro2::TokenTree = input.parse()?;
            return Err(syn::Error::new_spanned(
                &unknown,
                "\"implements\" attribute must have specification of the state generic in the macro.",
            ));
        }

        Ok(Implements {
            implemented_traits,
            state_generic,
        })
    }
}

pub struct GenericState {
    state: kw::state,
    equal: Token![=],
    ty: TypePath,
}

impl Parse for GenericState {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(GenericState {
            state: input.parse()?,
            equal: input.parse()?,
            ty: input.parse()?,
        })
    }
}

fn try_parse<T: Parse>(
    input: syn::parse::ParseStream,
    seen: Option<&mut Option<proc_macro2::Span>>,
    error_count: &mut usize,
) -> Option<syn::Result<T>> {
    let fork = input.fork();

    // Try to parse the fork, to see if there is a valid T.
    let parsed = match fork.parse::<T>() {
        Ok(parsed) => parsed,
        Err(_) => {
            *error_count += 1;
            return None;
        }
    };

    // If tracking is enabled, check for duplicates and record the span.
    if let Some(seen_ref) = seen {
        if let Some(span) = *seen_ref {
            // If we have seen this attribute before, return an error without advancing.
            return Some(Err(syn::Error::new(span, "Duplicate attribute")));
        }

        *seen_ref = Some(input.span());
    }

    input.advance_to(&fork);
    Some(Ok(parsed))
}
