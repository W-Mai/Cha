//! cross_boundary_chain — a function reaches several levels deep into a
//! parameter whose type comes from an external module. The traversal is
//! crossing a library's public API into its internal shape, which
//! tightly couples the function to that library's field layout.
//!
//! Complements the existing `message_chain` smell (which fires on any
//! long chain regardless of where the root came from). This one fires
//! only when the chain's root is a third-party type — narrower, but a
//! much stronger signal of abstraction leak.
//!
//! Inputs: `chain_depth`, `parameter_types` + `parameter_names`,
//! `external_refs`. Zero parser changes.

use std::collections::HashSet;
use std::path::Path;

use cha_core::{Finding, FunctionInfo, Location, Severity, SmellCategory, TypeOrigin};

use crate::project_index::ProjectIndex;

const SMELL: &str = "cross_boundary_chain";
const MIN_DEPTH: usize = 3;

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let workspace_crates = workspace_crate_names(index);
    let mut findings = Vec::new();
    for (path, model) in index.models() {
        for f in &model.functions {
            if f.chain_depth < MIN_DEPTH {
                continue;
            }
            let Some((param_name, module)) = find_external_traversed_param(f, &workspace_crates)
            else {
                continue;
            };
            findings.push(build_finding(path, f, param_name, module, f.chain_depth));
        }
    }
    findings
}

/// Derive workspace-internal crate names from every model path, so
/// deep access into a sibling project crate (`cha_core::Finding`) is
/// not flagged as a third-party boundary crossing. Shares the
/// convention used by `leaky_public_signature`.
fn workspace_crate_names(index: &ProjectIndex) -> HashSet<String> {
    let mut names = HashSet::new();
    for (path, _) in index.models() {
        let Some(first) = path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .find(|s| *s != "." && *s != "..")
        else {
            continue;
        };
        names.insert(first.replace('-', "_"));
    }
    names
}

/// Return the first (name, module) pair for a parameter that is both
/// externally-typed **and** referenced by name in the function body.
/// Requiring the body reference cuts the false positives where an
/// external param simply exists in the signature but isn't touched.
fn find_external_traversed_param<'a>(
    f: &'a FunctionInfo,
    workspace_crates: &HashSet<String>,
) -> Option<(&'a str, &'a str)> {
    for (name, ty) in f.parameter_names.iter().zip(f.parameter_types.iter()) {
        let TypeOrigin::External(module) = &ty.origin else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        // Workspace-internal crates (e.g. `cha_core` inside the Cha
        // repo) are not a third-party boundary — skip.
        let root = module.split("::").next().unwrap_or(module);
        if workspace_crates.contains(root) {
            continue;
        }
        if !f.external_refs.iter().any(|r| r == name) {
            continue;
        }
        return Some((name.as_str(), module.as_str()));
    }
    None
}

fn build_finding(
    path: &Path,
    f: &FunctionInfo,
    param_name: &str,
    module: &str,
    depth: usize,
) -> Finding {
    let module_hint = if module.is_empty() {
        "an external module".to_string()
    } else {
        format!("`{module}`")
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
            "Function `{}` chains {} levels deep into `{}` (from {}) — each link depends on the library's internal shape, not its public API",
            f.name, depth, param_name, module_hint,
        ),
        suggested_refactorings: vec![
            format!(
                "Extract a local helper that accepts `{}` and exposes a narrow, project-owned view — restricts the coupling surface to one place",
                param_name
            ),
            format!(
                "Or introduce an Adapter wrapping `{}` so the rest of the codebase stops knowing about the external type's layout",
                module,
            ),
        ],
        actual_value: Some(depth as f64),
        threshold: Some(MIN_DEPTH as f64),
        risk_score: None,
    }
}

#[cfg(test)]
mod tests;
