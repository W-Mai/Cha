//! Parameter position inconsistency: the same domain type appears at
//! different parameter positions across functions that accept it. Example:
//!
//! ```text
//! fn send(user: User, msg: Message) { ... }       // User at #1
//! fn notify(msg: Message, user: User) { ... }     // User at #2
//! ```
//!
//! Not strictly a bug, but a trap — refactors that swap argument order are
//! exactly the kind of change that slips past review. Reliably flags
//! inconsistent callable shapes across files.
//!
//! Only runs on non-primitive, origin-resolved types — `String`/`i32` are
//! allowed to float because positional convention there is weaker.

use std::collections::HashMap;
use std::path::Path;

use cha_core::{Finding, FunctionInfo, Location, Severity, SmellCategory, TypeOrigin, TypeRef};

use crate::project_index::ProjectIndex;

const SMELL: &str = "parameter_position_inconsistency";
const MIN_FUNCTIONS: usize = 3;

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let sites = collect_type_usage_sites(index);
    let mut findings = Vec::new();
    for (type_name, usages) in &sites {
        if usages.len() < MIN_FUNCTIONS {
            continue;
        }
        let positions: std::collections::HashSet<usize> =
            usages.iter().map(|u| u.position).collect();
        if positions.len() < 2 {
            // All usages agree on position — no inconsistency.
            continue;
        }
        findings.extend(build_findings_for_type(type_name, usages));
    }
    findings
}

struct UsageSite<'a> {
    path: &'a Path,
    function: &'a FunctionInfo,
    position: usize,
    arity: usize,
}

/// Scan every function signature. For each non-primitive parameter type,
/// record (function, position, arity).
fn collect_type_usage_sites<'a>(index: &'a ProjectIndex) -> HashMap<&'a str, Vec<UsageSite<'a>>> {
    let mut sites: HashMap<&str, Vec<UsageSite>> = HashMap::new();
    for (path, model) in index.models() {
        for f in &model.functions {
            let arity = f.parameter_types.len();
            for (idx, t) in f.parameter_types.iter().enumerate() {
                if !is_interesting(t) {
                    continue;
                }
                // Skip "self" positions — methods can't reorder their receiver.
                if idx == 0 && is_self_parameter(t) {
                    continue;
                }
                sites.entry(t.name.as_str()).or_default().push(UsageSite {
                    path: path.as_path(),
                    function: f,
                    position: idx + 1,
                    arity,
                });
            }
        }
    }
    sites
}

fn is_interesting(t: &TypeRef) -> bool {
    if matches!(t.origin, TypeOrigin::Primitive | TypeOrigin::Unknown) {
        return false;
    }
    // Drop mutable-ref out-parameters: `&mut Vec<Finding>` and friends are
    // conventionally written last regardless of the type's position elsewhere,
    // which otherwise generates a lot of "Finding at #N" noise when Finding
    // also appears as an owned param.
    if is_mutable_reference(&t.raw) {
        return false;
    }
    true
}

fn is_mutable_reference(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    trimmed.starts_with("&mut ")
        || trimmed.starts_with("&mut\t")
        || trimmed.starts_with("*mut ")
        || trimmed.starts_with("*mut\t")
}

fn is_self_parameter(t: &TypeRef) -> bool {
    // Rust-style receiver sneaks in as a parameter with a typical raw shape.
    let raw = t.raw.as_str();
    raw == "self" || raw == "&self" || raw == "&mut self" || raw.starts_with("self:")
}

/// Summarise each offending type with one finding on each offending call site,
/// so the IDE underlines the actual parameter-position mismatch. A "canonical"
/// position is chosen (the most common), and every function that disagrees
/// gets flagged.
fn build_findings_for_type(type_name: &str, usages: &[UsageSite<'_>]) -> Vec<Finding> {
    let canonical = canonical_position(usages);
    usages
        .iter()
        .filter(|u| u.position != canonical)
        .map(|u| build_finding(type_name, u, canonical))
        .collect()
}

fn canonical_position(usages: &[UsageSite<'_>]) -> usize {
    let mut counts: HashMap<usize, usize> = HashMap::new();
    for u in usages {
        *counts.entry(u.position).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(p, _)| p)
        .unwrap_or(1)
}

fn build_finding(type_name: &str, site: &UsageSite<'_>, canonical: usize) -> Finding {
    Finding {
        smell_name: SMELL.into(),
        category: SmellCategory::Couplers,
        severity: Severity::Hint,
        location: Location {
            path: site.path.to_path_buf(),
            start_line: site.function.start_line,
            start_col: site.function.name_col,
            end_line: site.function.start_line,
            end_col: site.function.name_end_col,
            name: Some(site.function.name.clone()),
        },
        message: format!(
            "Function `{}` takes `{}` at position #{} / {}, but most other callers have it at position #{} — inconsistent parameter order is a refactor hazard",
            site.function.name, type_name, site.position, site.arity, canonical,
        ),
        suggested_refactorings: vec![
            format!(
                "Move `{}` to position #{} to match the project's dominant convention",
                type_name, canonical
            ),
            "Or introduce a struct that carries the grouped arguments (Parameter Object)".into(),
        ],
        actual_value: Some(site.position as f64),
        threshold: Some(canonical as f64),
    }
}

#[cfg(test)]
mod tests;
