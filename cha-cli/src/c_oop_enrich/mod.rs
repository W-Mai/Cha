//! Cross-file C OOP method attribution.
//!
//! C has no syntactic "method", but projects follow conventions: a struct
//! `foo_t` paired with `foo_*` prefixed functions whose first parameter is
//! `foo_t *`. This pass runs after all files parse, walks the
//! project-wide function/struct universe, and writes back:
//!
//! - `ClassInfo.method_count` / `has_behavior` — incremented for each
//!   function attributed to the struct.
//! - `FunctionInfo.is_exported` — tightened: a non-static function in a `.c`
//!   file that isn't declared in any project `.h` is treated as internal
//!   linkage in spirit, even if the C compiler would link it externally.
//!
//! The "language-aware" surgery is kept behind `SourceModel.language` checks
//! so models for other languages pass through unchanged. Runs inside
//! `ProjectIndex::parse` so every index-backed detector sees the enriched
//! view.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cha_core::{SourceModel, SymbolIndex};

/// Entry point — mutate the shared set of parsed models in place.
pub fn enrich_c_oop(models: &mut [(PathBuf, SourceModel)]) {
    if !models.iter().any(|(_, m)| is_c_like(m)) {
        return;
    }
    let index = build_index(models);
    let attributions = attribute_methods(models, &index);
    let exported_in_headers = collect_header_exports(models);
    write_back(models, &attributions, &exported_in_headers);
}

/// `SymbolIndex` twin of `enrich_c_oop` — same algorithm, but walks the
/// compact symbol-level view instead of full `SourceModel`. Used by
/// `cha deps` whose inputs come from `cached_symbols`.
pub fn enrich_c_oop_symbols(indices: &mut [(PathBuf, SymbolIndex)]) {
    if !indices.iter().any(|(_, s)| is_c_like_sym(s)) {
        return;
    }
    let index = build_index_from_symbols(indices);
    let attributions = attribute_methods_from_symbols(indices, &index);
    let exported_in_headers = collect_header_exports_from_symbols(indices);
    write_back_symbols(indices, &attributions, &exported_in_headers);
}

/// Method attribution exposed for consumers that want to know *which*
/// functions attach to each C struct (not just the count). Returns map:
/// struct name → `(path, fn_name, is_exported)` for every function
/// attributed to that struct. Kept alongside the SymbolIndex variant so
/// SourceModel-based callers (future ProjectIndex work, plugins) don't
/// need to downgrade through SymbolIndex first.
#[allow(dead_code)]
pub fn attribute_methods_by_name(
    models: &[(PathBuf, SourceModel)],
) -> HashMap<String, Vec<(PathBuf, String, bool)>> {
    let mut out: HashMap<String, Vec<(PathBuf, String, bool)>> = HashMap::new();
    if !models.iter().any(|(_, m)| is_c_like(m)) {
        return out;
    }
    let index = build_index(models);
    for (path, model) in models {
        if !is_c_like(model) {
            continue;
        }
        for f in &model.functions {
            if let Some(key) = attribute_one(f, &index) {
                out.entry(key)
                    .or_default()
                    .push((path.clone(), f.name.clone(), f.is_exported));
            }
        }
    }
    out
}

/// `SymbolIndex` twin of `attribute_methods_by_name`. Used by deps'
/// `--detail` path so it never has to touch `SourceModel`.
pub fn attribute_methods_by_name_from_symbols(
    indices: &[(PathBuf, SymbolIndex)],
) -> HashMap<String, Vec<(PathBuf, String, bool)>> {
    let mut out: HashMap<String, Vec<(PathBuf, String, bool)>> = HashMap::new();
    if !indices.iter().any(|(_, s)| is_c_like_sym(s)) {
        return out;
    }
    let index = build_index_from_symbols(indices);
    for (path, s) in indices {
        if !is_c_like_sym(s) {
            continue;
        }
        for f in &s.functions {
            let first = f.parameter_type_names.first().map(String::as_str);
            if let Some(key) = attribute_one_raw(&f.name, first, &index) {
                out.entry(key)
                    .or_default()
                    .push((path.clone(), f.name.clone(), f.is_exported));
            }
        }
    }
    out
}

// ── Index construction ────────────────────────────────────────────────────

/// Attribute by struct name — forward declarations and full definitions of
/// the same struct share attribution. Matches C project reality: a struct
/// is forward-declared in a types header and defined in a private header;
/// both ClassInfo instances should count every attributed method.
type ClassKey = String;

struct Index {
    /// Type-tokens → owning class name. Populated from struct names *and*
    /// typedef aliases pointing at them.
    type_to_class: HashMap<Vec<String>, ClassKey>,
    /// Global prefix → classes that claim it. A class claims every
    /// non-empty prefix of (its own name tokens + its aliases' tokens).
    /// When a function name is attributed, we find the longest prefix it
    /// shares with this map — that localises the function to the most
    /// specific naming family, not just the first-word family.
    prefix_to_owners: HashMap<Vec<String>, HashSet<ClassKey>>,
    /// Direct parent edges (C inheritance via first-field-is-base). Keyed
    /// by ClassKey so we can walk upward from a derived struct.
    parent_of: HashMap<ClassKey, ClassKey>,
}

// cha:ignore duplicate_code
fn build_index(models: &[(PathBuf, SourceModel)]) -> Index {
    let mut type_to_class: HashMap<Vec<String>, ClassKey> = HashMap::new();
    let mut prefix_to_owners: HashMap<Vec<String>, HashSet<ClassKey>> = HashMap::new();

    register_structs(models, &mut type_to_class, &mut prefix_to_owners);
    register_aliases(models, &mut type_to_class, &mut prefix_to_owners);
    let parent_of = build_parent_map(models, &type_to_class);

    Index {
        type_to_class,
        prefix_to_owners,
        parent_of,
    }
}

/// Pass 1: every struct claims its own name tokens + every non-empty
/// prefix of them. Multiple declarations (forward + definition) share
/// a ClassKey, so attribution to "the struct name" hits both.
fn register_structs(
    models: &[(PathBuf, SourceModel)],
    type_to_class: &mut HashMap<Vec<String>, ClassKey>,
    prefix_to_owners: &mut HashMap<Vec<String>, HashSet<ClassKey>>,
) {
    for (_, model) in models {
        if !is_c_like(model) {
            continue;
        }
        for c in &model.classes {
            let tokens = tokenize(&c.name);
            if tokens.is_empty() {
                continue;
            }
            type_to_class
                .entry(tokens.clone())
                .or_insert(c.name.clone());
            for prefix in candidate_prefixes(&tokens) {
                prefix_to_owners
                    .entry(prefix)
                    .or_default()
                    .insert(c.name.clone());
            }
        }
    }
}

/// Pass 2: typedef aliases contribute additional prefix claims to their
/// target class. `typedef struct _foo foo_t;` means functions prefixed
/// `foo_*` also belong to `_foo`.
fn register_aliases(
    models: &[(PathBuf, SourceModel)],
    type_to_class: &mut HashMap<Vec<String>, ClassKey>,
    prefix_to_owners: &mut HashMap<Vec<String>, HashSet<ClassKey>>,
) {
    for (_, model) in models {
        if !is_c_like(model) {
            continue;
        }
        for (alias, original) in &model.type_aliases {
            let alias_tokens = tokenize(alias);
            let original_tokens = tokenize(original);
            if alias_tokens.is_empty() {
                continue;
            }
            let Some(key) = type_to_class
                .get(&original_tokens)
                .or_else(|| type_to_class.get(&alias_tokens))
                .cloned()
            else {
                // alias points at something we don't know as a class (e.g.
                // `typedef uint32_t tag_t;` — primitive alias, no struct)
                continue;
            };
            type_to_class
                .entry(alias_tokens.clone())
                .or_insert(key.clone());
            for prefix in candidate_prefixes(&alias_tokens) {
                prefix_to_owners
                    .entry(prefix)
                    .or_default()
                    .insert(key.clone());
            }
        }
    }
}

/// Pass 3: inheritance via "first field is a value of the parent type".
/// The parser records that in `ClassInfo.parent_name`; we normalise the
/// raw name to a canonical ClassKey via type_to_class so the ancestor
/// walk works regardless of whether the declaration used the struct
/// tag or its typedef alias.
fn build_parent_map(
    models: &[(PathBuf, SourceModel)],
    type_to_class: &HashMap<Vec<String>, ClassKey>,
) -> HashMap<ClassKey, ClassKey> {
    let mut parent_of: HashMap<ClassKey, ClassKey> = HashMap::new();
    for (_, model) in models {
        if !is_c_like(model) {
            continue;
        }
        for c in &model.classes {
            let Some(parent_raw) = c.parent_name.as_deref() else {
                continue;
            };
            let parent_tokens = tokenize(parent_raw);
            if parent_tokens.is_empty() {
                continue;
            }
            let Some(parent_key) = type_to_class.get(&parent_tokens) else {
                continue;
            };
            if *parent_key == c.name {
                continue; // avoid self-loops from typedef struct _foo foo;
            }
            parent_of
                .entry(c.name.clone())
                .or_insert_with(|| parent_key.clone());
        }
    }
    parent_of
}

// ── Attribution ────────────────────────────────────────────────────────────

fn attribute_methods(models: &[(PathBuf, SourceModel)], index: &Index) -> HashMap<ClassKey, usize> {
    let mut counts: HashMap<ClassKey, usize> = HashMap::new();
    for (_, model) in models {
        if !is_c_like(model) {
            continue;
        }
        for f in &model.functions {
            let Some(key) = attribute_one(f, index) else {
                continue;
            };
            *counts.entry(key).or_default() += 1;
        }
    }
    counts
}

/// Attribute a function to the struct it morally "belongs" to.
///
/// Two-gate design:
/// 1. **Param gate** — first-parameter type must resolve to a known struct
///    (the `target`). This confines attribution to functions whose first
///    arg looks like a `self` pointer.
/// 2. **Longest-prefix gate** — the function-name tokens must start with
///    some prefix registered by *any* struct; we pick the *longest* such
///    prefix. That prefix's owners form the candidate set.
///
/// The returned owner is then chosen from candidates so that
/// `owner == target OR target ∈ ancestors(owner)` — i.e. the function is
/// attributed to the most specific subclass whose naming family matches,
/// as long as its first parameter is an upcast of that subclass. This
/// reflects C "OOP" done by embedding a parent struct as the first field
/// and passing `&derived->parent` around.
fn attribute_one(f: &cha_core::FunctionInfo, index: &Index) -> Option<ClassKey> {
    let first = f.parameter_types.first()?;
    attribute_one_raw(&f.name, Some(first.raw.as_str()), index)
}

/// Attribution on raw type text — the real brain. Both the SourceModel
/// path (`FunctionInfo.parameter_types[0].raw`) and the SymbolIndex path
/// (`FunctionSymbol.parameter_type_names[0]`) funnel through here.
fn attribute_one_raw(
    fn_name: &str,
    first_param_raw: Option<&str>,
    index: &Index,
) -> Option<ClassKey> {
    let bare = normalize_type_raw(first_param_raw?);
    let param_tokens = tokenize(&bare);
    if param_tokens.is_empty() {
        return None;
    }
    let target = index.type_to_class.get(&param_tokens)?;

    let fn_tokens = tokenize(fn_name);
    let owners = longest_prefix_owners(&fn_tokens, &index.prefix_to_owners)?;

    if owners.contains(target) {
        return Some(target.clone());
    }
    owners
        .iter()
        .find(|owner| ancestor_chain(owner, &index.parent_of).contains(target.as_str()))
        .cloned()
}

fn longest_prefix_owners<'a>(
    fn_tokens: &[String],
    index: &'a HashMap<Vec<String>, HashSet<ClassKey>>,
) -> Option<&'a HashSet<ClassKey>> {
    (1..=fn_tokens.len())
        .rev()
        .find_map(|len| index.get(&fn_tokens[..len].to_vec()))
}

/// Walk `owner`'s parent chain, collecting every ancestor's ClassKey.
/// Bounded by project size to defend against accidental cycles.
fn ancestor_chain<'a>(
    owner: &ClassKey,
    parent_of: &'a HashMap<ClassKey, ClassKey>,
) -> HashSet<&'a str> {
    let mut seen = HashSet::new();
    let mut cur = parent_of.get(owner);
    let mut depth = 0;
    while let Some(p) = cur {
        if !seen.insert(p.as_str()) {
            break; // cycle
        }
        depth += 1;
        if depth > 32 {
            break;
        }
        cur = parent_of.get(p);
    }
    seen
}

// ── .h export set ─────────────────────────────────────────────────────────

fn collect_header_exports(models: &[(PathBuf, SourceModel)]) -> HashSet<String> {
    let mut set = HashSet::new();
    for (path, model) in models {
        if !is_c_like(model) {
            continue;
        }
        if !is_header_path(path) {
            continue;
        }
        for f in &model.functions {
            set.insert(f.name.clone());
        }
    }
    set
}

// ── Write back ────────────────────────────────────────────────────────────

// cha:ignore duplicate_code
fn write_back(
    models: &mut [(PathBuf, SourceModel)],
    attributions: &HashMap<ClassKey, usize>,
    exported_in_headers: &HashSet<String>,
) {
    for (path, model) in models.iter_mut() {
        if !is_c_like(model) {
            continue;
        }
        if !is_header_path(path) {
            tighten_exports(model, exported_in_headers);
        }
        apply_attributions(model, attributions);
    }
}

/// In a .c file, demote non-static functions that never appear in any
/// project header declaration to `is_exported = false`. Those are
/// "forgot to write static" internal helpers — linker lets them out
/// but no header exposes them, so callers outside this TU shouldn't
/// treat them as part of the public API.
fn tighten_exports(model: &mut SourceModel, exported_in_headers: &HashSet<String>) {
    for f in &mut model.functions {
        if f.is_exported && !exported_in_headers.contains(&f.name) {
            f.is_exported = false;
        }
    }
}

/// Apply method-count attribution to every ClassInfo sharing a name
/// with an attributed key. Forward declarations and full definitions
/// of the same struct both receive the increment, so neither is
/// incorrectly flagged as lazy_class downstream.
fn apply_attributions(model: &mut SourceModel, attributions: &HashMap<ClassKey, usize>) {
    for c in &mut model.classes {
        if let Some(&added) = attributions.get(&c.name) {
            c.method_count += added;
            c.has_behavior = true;
        }
    }
}

// ── SymbolIndex parallel implementations ──────────────────────────────────
//
// Mirror of the SourceModel path — same index construction and
// attribution rules, but reading and writing `SymbolIndex` fields.

fn is_c_like_sym(s: &SymbolIndex) -> bool {
    matches!(s.language.as_str(), "c" | "cpp")
}

/// Intentional parallel of `build_index` — the SymbolIndex path stays
/// independent so `cha deps` never pulls in SourceModel. Shared `Index`
/// result + `attribute_one_raw` keep the actual rules single-sourced.
// cha:ignore duplicate_code
fn build_index_from_symbols(indices: &[(PathBuf, SymbolIndex)]) -> Index {
    let mut type_to_class: HashMap<Vec<String>, ClassKey> = HashMap::new();
    let mut prefix_to_owners: HashMap<Vec<String>, HashSet<ClassKey>> = HashMap::new();
    register_structs_sym(indices, &mut type_to_class, &mut prefix_to_owners);
    register_aliases_sym(indices, &mut type_to_class, &mut prefix_to_owners);
    let parent_of = build_parent_map_sym(indices, &type_to_class);
    Index {
        type_to_class,
        prefix_to_owners,
        parent_of,
    }
}

fn register_structs_sym(
    indices: &[(PathBuf, SymbolIndex)],
    type_to_class: &mut HashMap<Vec<String>, ClassKey>,
    prefix_to_owners: &mut HashMap<Vec<String>, HashSet<ClassKey>>,
) {
    for (_, s) in indices {
        if !is_c_like_sym(s) {
            continue;
        }
        for c in &s.classes {
            let tokens = tokenize(&c.name);
            if tokens.is_empty() {
                continue;
            }
            type_to_class
                .entry(tokens.clone())
                .or_insert_with(|| c.name.clone());
            for prefix in candidate_prefixes(&tokens) {
                prefix_to_owners
                    .entry(prefix)
                    .or_default()
                    .insert(c.name.clone());
            }
        }
    }
}

fn register_aliases_sym(
    indices: &[(PathBuf, SymbolIndex)],
    type_to_class: &mut HashMap<Vec<String>, ClassKey>,
    prefix_to_owners: &mut HashMap<Vec<String>, HashSet<ClassKey>>,
) {
    for (_, s) in indices {
        if !is_c_like_sym(s) {
            continue;
        }
        for (alias, original) in &s.type_aliases {
            let alias_tokens = tokenize(alias);
            let original_tokens = tokenize(original);
            if alias_tokens.is_empty() {
                continue;
            }
            let Some(key) = type_to_class
                .get(&original_tokens)
                .or_else(|| type_to_class.get(&alias_tokens))
                .cloned()
            else {
                continue;
            };
            type_to_class
                .entry(alias_tokens.clone())
                .or_insert_with(|| key.clone());
            for prefix in candidate_prefixes(&alias_tokens) {
                prefix_to_owners
                    .entry(prefix)
                    .or_default()
                    .insert(key.clone());
            }
        }
    }
}

fn build_parent_map_sym(
    indices: &[(PathBuf, SymbolIndex)],
    type_to_class: &HashMap<Vec<String>, ClassKey>,
) -> HashMap<ClassKey, ClassKey> {
    let mut parent_of: HashMap<ClassKey, ClassKey> = HashMap::new();
    for (_, s) in indices {
        if !is_c_like_sym(s) {
            continue;
        }
        for c in &s.classes {
            let Some(parent_raw) = c.parent_name.as_deref() else {
                continue;
            };
            let parent_tokens = tokenize(parent_raw);
            if parent_tokens.is_empty() {
                continue;
            }
            let Some(parent_key) = type_to_class.get(&parent_tokens) else {
                continue;
            };
            if *parent_key == c.name {
                continue;
            }
            parent_of
                .entry(c.name.clone())
                .or_insert_with(|| parent_key.clone());
        }
    }
    parent_of
}

fn attribute_methods_from_symbols(
    indices: &[(PathBuf, SymbolIndex)],
    index: &Index,
) -> HashMap<ClassKey, usize> {
    let mut counts: HashMap<ClassKey, usize> = HashMap::new();
    for (_, s) in indices {
        if !is_c_like_sym(s) {
            continue;
        }
        for f in &s.functions {
            let first = f.parameter_type_names.first().map(String::as_str);
            let Some(key) = attribute_one_raw(&f.name, first, index) else {
                continue;
            };
            *counts.entry(key).or_default() += 1;
        }
    }
    counts
}

fn collect_header_exports_from_symbols(indices: &[(PathBuf, SymbolIndex)]) -> HashSet<String> {
    let mut set = HashSet::new();
    for (path, s) in indices {
        if !is_c_like_sym(s) || !is_header_path(path) {
            continue;
        }
        for f in &s.functions {
            set.insert(f.name.clone());
        }
    }
    set
}

/// Intentional parallel of `write_back` for SymbolIndex. Same reason as
/// `build_index_from_symbols` — two storage types need two entry points.
// cha:ignore duplicate_code
fn write_back_symbols(
    indices: &mut [(PathBuf, SymbolIndex)],
    attributions: &HashMap<ClassKey, usize>,
    exported_in_headers: &HashSet<String>,
) {
    for (path, s) in indices.iter_mut() {
        if !is_c_like_sym(s) {
            continue;
        }
        if !is_header_path(path) {
            tighten_exports_sym(s, exported_in_headers);
        }
        apply_attributions_sym(s, attributions);
    }
}

fn tighten_exports_sym(s: &mut SymbolIndex, exported_in_headers: &HashSet<String>) {
    for f in &mut s.functions {
        if f.is_exported && !exported_in_headers.contains(&f.name) {
            f.is_exported = false;
        }
    }
}

fn apply_attributions_sym(s: &mut SymbolIndex, attributions: &HashMap<ClassKey, usize>) {
    for c in &mut s.classes {
        if let Some(&added) = attributions.get(&c.name) {
            c.method_count += added;
            c.has_behavior = true;
        }
    }
}

// ── Tokenisation ──────────────────────────────────────────────────────────

/// Split a C identifier into lowercase word-tokens. Handles snake_case,
/// PascalCase, camelCase, mixed (`Foo_Bar`), and consecutive-uppercase
/// acronyms (`HTTPRequest` → `["http", "request"]`).
pub(crate) fn tokenize(name: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for segment in name.split('_') {
        if segment.is_empty() {
            continue;
        }
        split_case(segment, &mut tokens);
    }
    tokens
}

fn split_case(segment: &str, out: &mut Vec<String>) {
    let chars: Vec<char> = segment.chars().collect();
    let mut cur = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_uppercase() {
            let run_start = i;
            while i < chars.len() && chars[i].is_ascii_uppercase() {
                i += 1;
            }
            emit_uppercase_run(&chars, run_start, i, &mut cur, out);
        } else {
            cur.push(c.to_ascii_lowercase());
            i += 1;
        }
    }
    flush(&mut cur, out);
}

/// Emit tokens for one contiguous uppercase run `chars[start..end]`.
/// Three cases:
/// - Length 1, followed by lowercase → PascalCase word boundary.
/// - Length > 1 followed by lowercase (e.g. `HTTPRequest`) → acronym
///   minus its last letter as one token, then the last letter starts
///   the next word.
/// - Otherwise (end of identifier, or run-then-underscore) → the whole
///   run is a single acronym token.
fn emit_uppercase_run(
    chars: &[char],
    start: usize,
    end: usize,
    cur: &mut String,
    out: &mut Vec<String>,
) {
    let run_len = end - start;
    let followed_by_lower = end < chars.len() && chars[end].is_ascii_lowercase();
    flush(cur, out);
    if run_len > 1 && followed_by_lower {
        for c in &chars[start..end - 1] {
            cur.push(c.to_ascii_lowercase());
        }
        flush(cur, out);
        cur.push(chars[end - 1].to_ascii_lowercase());
    } else if run_len == 1 {
        cur.push(chars[start].to_ascii_lowercase());
    } else {
        for c in &chars[start..end] {
            cur.push(c.to_ascii_lowercase());
        }
        flush(cur, out);
    }
}

fn flush(cur: &mut String, out: &mut Vec<String>) {
    if !cur.is_empty() {
        out.push(std::mem::take(cur));
    }
}

/// All non-empty prefixes of a token sequence.
fn candidate_prefixes(tokens: &[String]) -> Vec<Vec<String>> {
    (1..=tokens.len()).map(|n| tokens[..n].to_vec()).collect()
}

/// Strip `const`/`volatile`/`static`/`restrict` qualifiers, `struct `/`union `/
/// `enum ` prefixes, all `*`/`&`/whitespace — leaving the bare type name.
fn normalize_type_raw(raw: &str) -> String {
    let mut s = raw.to_string();
    for kw in &[
        "const ",
        "volatile ",
        "static ",
        "restrict ",
        "struct ",
        "union ",
        "enum ",
    ] {
        while let Some(pos) = s.find(kw) {
            s.replace_range(pos..pos + kw.len(), "");
        }
    }
    s.chars()
        .filter(|c| !matches!(c, '*' | '&') && !c.is_whitespace())
        .collect()
}

// ── Language / path helpers ───────────────────────────────────────────────

fn is_c_like(model: &SourceModel) -> bool {
    matches!(model.language.as_str(), "c" | "cpp")
}

fn is_header_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e, "h" | "hxx" | "hpp"))
}

#[cfg(test)]
mod tests;
