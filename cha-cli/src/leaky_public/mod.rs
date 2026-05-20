//! Leaky public signature: exported functions that expose a third-party
//! crate's type in their parameters or return type. The external type
//! becomes part of the function's public contract — every caller has to
//! know about the dep even to compile.
//!
//! Uses the shared `ProjectIndex` so it can build a set of workspace-
//! internal crates (derived from file paths) and skip types originating
//! from them. `cha-core → cha-parser` type flow inside one workspace is
//! not a public-API leak.

use std::path::Path;

use cha_core::{
    Finding, FunctionInfo, Location, ProjectQuery, Severity, SmellCategory, TypeOrigin, TypeRef,
};

use crate::project_index::ProjectIndex;

const SMELL: &str = "leaky_public_signature";

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, model) in index.models() {
        for f in &model.functions {
            if !f.is_exported {
                continue;
            }
            if let Some((t, pos)) = first_leaky_type(f, index) {
                findings.push(build_finding(path, f, t, pos));
            }
        }
    }
    findings
}

enum Position {
    Return,
    Param(usize),
}

fn first_leaky_type<'a>(
    f: &'a FunctionInfo,
    index: &ProjectIndex,
) -> Option<(&'a TypeRef, Position)> {
    if let Some(ret) = &f.return_type
        && index.is_third_party(ret)
    {
        return Some((ret, Position::Return));
    }
    for (idx, t) in f.parameter_types.iter().enumerate() {
        if index.is_third_party(t) {
            return Some((t, Position::Param(idx + 1)));
        }
    }
    None
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
