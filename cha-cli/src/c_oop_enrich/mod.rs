//! Cross-file C OOP method attribution.
//!
//! C has no syntactic "method", but projects follow conventions: a struct
//! `foo_t` with functions `foo_xxx(foo_t *self, ...)`. This pass runs after
//! all files parse, walks the project-wide function/struct universe, and
//! writes back:
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

use cha_core::SourceModel;

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
    /// Candidate function-name prefix sets per class name. Each candidate
    /// is a token sequence; a function matches if its own tokens start
    /// with any candidate.
    prefixes: HashMap<ClassKey, HashSet<Vec<String>>>,
}

fn build_index(models: &[(PathBuf, SourceModel)]) -> Index {
    let mut type_to_class: HashMap<Vec<String>, ClassKey> = HashMap::new();
    let mut prefixes: HashMap<ClassKey, HashSet<Vec<String>>> = HashMap::new();

    // 1) Register every struct by its own name. Multiple declarations of
    //    the same struct (forward + definition) share a ClassKey.
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
            prefixes
                .entry(c.name.clone())
                .or_default()
                .extend(candidate_prefixes(&tokens));
        }
    }

    // 2) Layer typedef aliases on top: `typedef struct _foo foo_t;` means
    //    prefixes from `foo_t` also claim the `_foo` struct. Look up the
    //    aliased name in type_to_class and merge prefixes onto that class.
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
            let target = type_to_class
                .get(&original_tokens)
                .or_else(|| type_to_class.get(&alias_tokens))
                .cloned();
            let Some(key) = target else {
                // Alias points at something we don't know as a class (e.g.
                // `typedef uint32_t tag_t;` — primitive alias, no struct).
                continue;
            };
            type_to_class
                .entry(alias_tokens.clone())
                .or_insert(key.clone());
            prefixes
                .entry(key)
                .or_default()
                .extend(candidate_prefixes(&alias_tokens));
        }
    }

    Index {
        type_to_class,
        prefixes,
    }
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

fn attribute_one(f: &cha_core::FunctionInfo, index: &Index) -> Option<ClassKey> {
    let first = f.parameter_types.first()?;
    let bare = normalize_type_raw(&first.raw);
    let param_tokens = tokenize(&bare);
    if param_tokens.is_empty() {
        return None;
    }
    let key = index.type_to_class.get(&param_tokens)?;
    let candidates = index.prefixes.get(key)?;
    let fn_tokens = tokenize(&f.name);
    candidates
        .iter()
        .any(|p| fn_tokens.starts_with(p))
        .then(|| key.clone())
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

fn write_back(
    models: &mut [(PathBuf, SourceModel)],
    attributions: &HashMap<ClassKey, usize>,
    exported_in_headers: &HashSet<String>,
) {
    for (path, model) in models.iter_mut() {
        if !is_c_like(model) {
            continue;
        }

        // is_exported tighten — only in `.c`-ish source files. Header
        // declarations are already correctly `is_exported = true` from the
        // parser; we don't second-guess them.
        if !is_header_path(path) {
            for f in &mut model.functions {
                if f.is_exported && !exported_in_headers.contains(&f.name) {
                    f.is_exported = false;
                }
            }
        }

        // method_count / has_behavior from attribution. Every ClassInfo
        // with a matching name receives the same attribution — forward
        // declarations and full definitions of the same struct share it,
        // so neither gets incorrectly flagged as lazy_class.
        for c in &mut model.classes {
            if let Some(&added) = attributions.get(&c.name) {
                c.method_count += added;
                c.has_behavior = true;
            }
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
            // Acronym run: look ahead for consecutive uppercase followed by
            // a lowercase letter — split before the lowercase.
            let start = i;
            while i < chars.len() && chars[i].is_ascii_uppercase() {
                i += 1;
            }
            let run_end = i;
            // If the run is followed by lowercase, the last uppercase is the
            // start of a new PascalCase word.
            if i < chars.len() && chars[i].is_ascii_lowercase() && run_end - start > 1 {
                // Emit everything before the last uppercase as one token.
                flush(&mut cur, out);
                for c in &chars[start..run_end - 1] {
                    cur.push(c.to_ascii_lowercase());
                }
                flush(&mut cur, out);
                cur.push(chars[run_end - 1].to_ascii_lowercase());
            } else if run_end - start == 1 {
                // Single uppercase — PascalCase boundary.
                flush(&mut cur, out);
                cur.push(c.to_ascii_lowercase());
            } else {
                // Pure acronym (run not followed by lowercase, or at end).
                flush(&mut cur, out);
                for c in &chars[start..run_end] {
                    cur.push(c.to_ascii_lowercase());
                }
                flush(&mut cur, out);
            }
        } else {
            cur.push(c.to_ascii_lowercase());
            i += 1;
        }
    }
    flush(&mut cur, out);
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
