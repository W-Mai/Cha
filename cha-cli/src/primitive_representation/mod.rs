//! primitive_representation — a parameter (or return value) whose **name**
//! signals a domain concept (`user_id`, `email`, `status_code`, …) but
//! whose **type** is a raw primitive (`String`, `i32`, `bool`). The
//! signal says the author could have introduced a newtype / wrapper
//! struct to carry domain invariants, and didn't.
//!
//! Per-parameter, signature-only detector — runs against `ProjectIndex`
//! like the rest of the S8 family. Severity: `Hint`.
//!
//! Complements `primitive_obsession` (per-function ratio): this fires on
//! a single param as long as the name carries a business concept.

use std::path::Path;

use cha_core::{Finding, FunctionInfo, Location, Severity, SmellCategory, TypeOrigin, TypeRef};

use crate::project_index::ProjectIndex;

const SMELL: &str = "primitive_representation";

/// Tokens that mark a parameter name as "a domain concept" — if any of
/// these appears as a standalone word in the tokenised parameter name,
/// the param is in scope for the primitive-type check.
const BUSINESS_TOKENS: &[&str] = &[
    // identifiers
    "id",
    "ids",
    "uuid",
    "guid",
    "key",
    "pk",
    // contact / web
    "email",
    "phone",
    "fax",
    "url",
    "uri",
    "href",
    "endpoint",
    "host",
    "hostname",
    "domain",
    // filesystem
    "path",
    "filename",
    "filepath",
    "dir",
    "directory",
    // security
    "token",
    "secret",
    "password",
    "pass",
    "pwd",
    "salt",
    "hash",
    "signature",
    // enum-ish
    "status",
    "state",
    "kind",
    "category",
    "level",
    "tier",
    // temporal
    "timestamp",
    "ts",
    "date",
    "datetime",
    "time",
    "deadline",
    "expiry",
    "ttl",
    // quantity-with-unit (bare "count" / "size" / "len" stay in the
    // noise list below — they don't imply a domain type)
    "price",
    "cost",
    "currency",
    "amount",
    // locale
    "locale",
    "lang",
    "language",
    "country",
    "region",
    "timezone",
];

/// Tokens that override business-ness — if the name contains any of
/// these, it's generic framework vocabulary, not a domain concept.
/// When both lists match, noise wins (conservative: skip).
const NOISE_TOKENS: &[&str] = &[
    "data",
    "value",
    "input",
    "output",
    "arg",
    "args",
    "arguments",
    "param",
    "params",
    "result",
    "results",
    "buf",
    "buffer",
    "src",
    "dst",
    "source",
    "destination",
    "tmp",
    "temp",
    "len",
    "size",
    "count",
    "num",
    "ptr",
    "idx",
    "index",
    "iter",
    // single letters / tiny identifiers
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "m",
    "n",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
];

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, model) in index.models() {
        for f in &model.functions {
            if !f.is_exported {
                continue;
            }
            if let Some(finding) = check_function(path, f) {
                findings.push(finding);
            }
        }
    }
    findings
}

fn check_function(path: &Path, f: &FunctionInfo) -> Option<Finding> {
    let mut flagged: Vec<(usize, &str, &TypeRef)> = Vec::new();
    for (idx, ty) in f.parameter_types.iter().enumerate() {
        if !is_primitive_like(ty) {
            continue;
        }
        let Some(name) = f.parameter_names.get(idx) else {
            continue;
        };
        if !is_business_named(name) {
            continue;
        }
        flagged.push((idx + 1, name.as_str(), ty));
    }
    if flagged.is_empty() {
        return None;
    }
    Some(build_finding(path, f, &flagged))
}

/// Is this type a **scalar** primitive — the kind of type that, when
/// carrying a domain concept like `user_id`, begs to be wrapped in a
/// newtype?
///
/// The parser classifies containers and smart-pointers (`Vec`, `HashMap`,
/// `Arc`, `Box`, `Path`, `PathBuf`, `OsStr`, `Option`, `Result`, …) as
/// `TypeOrigin::Primitive` too — they're "language built-ins", not
/// domain types. But wrapping `Path` in a newtype **destroys** its
/// value as a path abstraction, so we exclude containers here.
fn is_primitive_like(t: &TypeRef) -> bool {
    if is_container_type(&t.name) {
        return false;
    }
    match t.origin {
        TypeOrigin::Primitive => is_scalar_name(&t.name),
        TypeOrigin::Unknown => is_scalar_name(&t.name),
        _ => false,
    }
}

/// Scalar primitives — the only types where "domain-named + raw" is a
/// smell. Integer widths, floats, booleans, chars, and raw string types
/// across the supported languages.
fn is_scalar_name(name: &str) -> bool {
    matches!(
        name,
        // Rust
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
        | "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
        | "f32" | "f64"
        | "bool" | "char" | "str" | "String"
        // Python / TS / C / Java
        | "int" | "long" | "short" | "byte" | "float" | "double"
        | "boolean" | "void" | "string" | "number" | "any" | "unknown"
    )
}

/// Container / wrapper / smart-pointer types that carry domain value of
/// their own — wrapping a `Path`-named `Path` param in a newtype is
/// noise, not an improvement.
fn is_container_type(name: &str) -> bool {
    matches!(
        name,
        "Vec"
            | "Option"
            | "Result"
            | "Box"
            | "Arc"
            | "Rc"
            | "Cell"
            | "RefCell"
            | "HashMap"
            | "HashSet"
            | "BTreeMap"
            | "BTreeSet"
            | "Path"
            | "PathBuf"
            | "OsStr"
            | "OsString"
            | "list"
            | "dict"
            | "set"
            | "tuple"
            | "bytes"
            | "bytearray"
    )
}

/// A parameter name is "business-named" when its token split contains a
/// business token and no noise token. Substring-safe: tokens must be
/// standalone words, so `widget_identifier` doesn't match `id` (tokens:
/// `widget`, `identifier`). Uses the project's shared snake/camel/PascalCase
/// tokeniser (`c_oop_enrich::tokenize`) so naming analyses stay consistent.
fn is_business_named(raw: &str) -> bool {
    let tokens = crate::c_oop_enrich::tokenize(raw);
    if tokens.iter().any(|t| NOISE_TOKENS.contains(&t.as_str())) {
        return false;
    }
    tokens.iter().any(|t| BUSINESS_TOKENS.contains(&t.as_str()))
}

fn build_finding(path: &Path, f: &FunctionInfo, flagged: &[(usize, &str, &TypeRef)]) -> Finding {
    let list: Vec<String> = flagged
        .iter()
        .map(|(pos, name, ty)| format!("`{name}: {}` (#{pos})", ty.name))
        .collect();
    let joined = list.join(", ");
    Finding {
        smell_name: SMELL.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Hint,
        location: Location {
            path: path.to_path_buf(),
            start_line: f.start_line,
            start_col: f.name_col,
            end_line: f.start_line,
            end_col: f.name_end_col,
            name: Some(f.name.clone()),
        },
        message: format!(
            "Function `{}` carries domain-named {} as raw primitive type(s) — consider introducing a newtype to preserve the invariant",
            f.name, joined,
        ),
        suggested_refactorings: vec![
            "Replace Data Value with Object: wrap each business-named primitive in its own struct"
                .into(),
            "If the type is used in many places, add a validated constructor (Value Object)".into(),
        ],
        actual_value: Some(flagged.len() as f64),
        threshold: Some(1.0),
        risk_score: None,
    }
}

#[cfg(test)]
mod tests;
