//! The same packet-header pipeline written twice, so the release assembly
//! of each can be compared.
//!
//! [`hand`] uses one struct per state and calls `transmute` directly.
//! [`groups`] uses one `#[typestate]` struct and `typestate-groups`. Every
//! function in one module has a twin in the other with the same body
//! shape and a `hand_` or `groups_` symbol prefix. `asm/` holds the
//! assembly of both, regenerated with `asm-compare/dump-asm.sh`.

#![no_std]

pub mod groups;
pub mod hand;
