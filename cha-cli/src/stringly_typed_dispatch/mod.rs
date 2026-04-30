//! stringly_typed_dispatch — a function's `switch`/`match` dispatches on
//! magic string or integer literals instead of a proper tagged type. The
//! dispatcher body looks like a lookup table that should have been an
//! enum.
//!
//! Complements `primitive_representation` (S8.2): that one looks at
//! signatures, this one looks at bodies.

use std::path::Path;

use cha_core::{ArmValue, Finding, FunctionInfo, Location, Severity, SmellCategory};

use crate::project_index::ProjectIndex;

const SMELL: &str = "stringly_typed_dispatch";
const MIN_LITERAL_ARMS: usize = 3;

pub fn detect(index: &ProjectIndex) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, model) in index.models() {
        for f in &model.functions {
            if let Some(finding) = check_function(path, f) {
                findings.push(finding);
            }
        }
    }
    findings
}

fn check_function(path: &Path, f: &FunctionInfo) -> Option<Finding> {
    let str_arms: Vec<&str> = f
        .switch_arm_values
        .iter()
        .filter_map(|a| match a {
            ArmValue::Str(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    let int_arms: Vec<i64> = f
        .switch_arm_values
        .iter()
        .filter_map(|a| match a {
            ArmValue::Int(n) => Some(*n),
            _ => None,
        })
        .collect();

    if str_arms.len() >= MIN_LITERAL_ARMS {
        return Some(build_finding(path, f, "string", &str_arms.join("\", \"")));
    }
    if int_arms.len() >= MIN_LITERAL_ARMS {
        let preview = int_arms
            .iter()
            .take(5)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Some(build_finding(path, f, "integer", &preview));
    }
    None
}

fn build_finding(path: &Path, f: &FunctionInfo, kind: &str, preview: &str) -> Finding {
    let quoted = if kind == "string" {
        format!("\"{preview}\"")
    } else {
        preview.to_string()
    };
    let refactoring = if kind == "string" {
        vec![
            "Replace the dispatched-on strings with an enum variant per arm".into(),
            "Introduce a parse/display pair so callers can still accept user strings but the internal flow is type-checked".into(),
        ]
    } else {
        vec![
            "Replace the magic integer arms with a `#[repr(u*)] enum` so the compiler checks exhaustiveness".into(),
            "If the integers follow a published spec, the enum keeps its representation via `#[repr]`".into(),
        ]
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
            "Function `{}` dispatches on {} literals ({}) — consider a tagged type instead of magic {}s",
            f.name, kind, quoted, kind,
        ),
        suggested_refactorings: refactoring,
        actual_value: Some(
            f.switch_arm_values
                .iter()
                .filter(|a| matches!(a, ArmValue::Str(_) | ArmValue::Int(_)))
                .count() as f64,
        ),
        threshold: Some(MIN_LITERAL_ARMS as f64),
        risk_score: None,
    }
}

#[cfg(test)]
mod tests;
