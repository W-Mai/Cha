//! TypeRef construction helpers shared across language parsers.
//!
//! In Phase 2 this is where per-language `ImportsMap` → `TypeOrigin`
//! resolution will live. For Phase 1 every parser emits Unknown origin;
//! the helper just centralises the `TypeRef { name, raw, origin }` shape.

use cha_core::{TypeOrigin, TypeRef};

/// Build a TypeRef with Unknown origin, auto-normalising `name` from `raw`
/// via a shared strip of refs/mut/pointer decorations.
pub fn unknown(raw: impl Into<String>) -> TypeRef {
    let raw = raw.into();
    let name = strip_decor(&raw);
    TypeRef {
        name,
        raw,
        origin: TypeOrigin::Unknown,
    }
}

/// Strip reference / pointer / mut decorations to recover the innermost
/// identifier. Generics are intentionally left alone at this layer — Phase 2
/// will add recursive unwrap for `Vec<…>` / `Option<…>` / `List[…]`.
fn strip_decor(raw: &str) -> String {
    raw.trim_start_matches('&')
        .trim_start_matches("mut ")
        .trim_start_matches('*')
        .trim_end_matches('*')
        .trim()
        .to_string()
}
