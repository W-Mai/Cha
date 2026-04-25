//! Leaky public signature: exported functions that expose a third-party
//! crate's type in their parameters or return type. The external type
//! becomes part of the function's public contract — every caller has to
//! know about the dep even to compile.
//!
//! Uses the shared `ProjectIndex` so it can build a set of workspace-
//! internal crates (derived from file paths) and skip types originating
//! from them. `cha-core → cha-parser` type flow inside one workspace is
//! not a public-API leak.

use std::collections::HashSet;
use std::path::Path;

use cha_core::{Finding, FunctionInfo, Location, Severity, SmellCategory, TypeOrigin, TypeRef};

use crate::project_index::ProjectIndex;

const SMELL: &str = "leaky_public_signature";

/// Modules treated as universal — part of the language prelude, not a dep
/// that consumers have to separately pull in.
const STDLIB_ROOTS: &[&str] = &[
    "std",
    "core",
    "alloc",
    "typing",
    "builtins",
    // proc_macro is Rust's built-in procedural-macro toolbox. Any proc-macro
    // crate has to take/return `TokenStream` by design — flagging it every
    // time is pure noise.
    "proc_macro",
    "proc_macro2",
];

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let workspace_crates = workspace_crate_names(index);
    let mut findings = Vec::new();
    for (path, model) in index.models() {
        for f in &model.functions {
            if !f.is_exported {
                continue;
            }
            if let Some((t, pos)) = first_leaky_type(f, &workspace_crates) {
                findings.push(build_finding(path, f, t, pos));
            }
        }
    }
    findings
}

/// Derive the set of workspace-internal crate names from every model's path.
/// `cha-parser/src/foo.rs` → `cha_parser`; the top-level directory's name
/// (with dashes flipped to underscores) matches how Rust imports these
/// crates, so `External("cha_parser")` coming out of the type-origin
/// resolver maps cleanly.
fn workspace_crate_names(index: &ProjectIndex) -> HashSet<String> {
    let mut names = HashSet::new();
    for (path, _) in index.models() {
        if let Some(krate) = first_path_component(path) {
            names.insert(krate.replace('-', "_"));
        }
    }
    names
}

fn first_path_component(path: &Path) -> Option<String> {
    // Skip `.` / `..` — analyze paths from the project root often come in as
    // `./cha-core/src/…` and the first real directory is what we want.
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .find(|s| *s != "." && *s != "..")
        .map(|s| s.to_string())
}

enum Position {
    Return,
    Param(usize),
}

fn first_leaky_type<'a>(
    f: &'a FunctionInfo,
    workspace_crates: &HashSet<String>,
) -> Option<(&'a TypeRef, Position)> {
    if let Some(ret) = &f.return_type
        && is_external_leak(ret, workspace_crates)
    {
        return Some((ret, Position::Return));
    }
    for (idx, t) in f.parameter_types.iter().enumerate() {
        if is_external_leak(t, workspace_crates) {
            return Some((t, Position::Param(idx + 1)));
        }
    }
    None
}

fn is_external_leak(t: &TypeRef, workspace_crates: &HashSet<String>) -> bool {
    let TypeOrigin::External(module) = &t.origin else {
        return false;
    };
    let root = module_root(module);
    if is_standard_root(root) {
        return false;
    }
    if workspace_crates.contains(root) {
        return false;
    }
    true
}

fn module_root(module: &str) -> &str {
    let first = module.split("::").next().unwrap_or(module);
    first.split('.').next().unwrap_or(first)
}

fn is_standard_root(root: &str) -> bool {
    STDLIB_ROOTS.contains(&root)
}

fn build_finding(path: &Path, f: &FunctionInfo, t: &TypeRef, pos: Position) -> Finding {
    let where_it = match pos {
        Position::Return => "return type".to_string(),
        Position::Param(i) => format!("parameter #{i}"),
    };
    let module = match &t.origin {
        TypeOrigin::External(m) => m.as_str(),
        _ => "external",
    };
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
            "Exported function `{}` has `{}` (from `{}`) in its {} — the third-party type becomes part of your public API",
            f.name, t.name, module, where_it
        ),
        suggested_refactorings: vec![
            format!(
                "Wrap `{}` in a local type before it crosses the module boundary",
                t.name
            ),
            "Or make the function non-public if only internal code needs it".into(),
        ],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests;
