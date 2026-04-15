use std::collections::HashMap;

use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect bidirectional imports between files (inappropriate intimacy).
/// Requires multi-file context: accumulates import data across files.
pub struct InappropriateIntimacyAnalyzer;

impl Default for InappropriateIntimacyAnalyzer {
    fn default() -> Self {
        Self
    }
}

impl Plugin for InappropriateIntimacyAnalyzer {
    fn name(&self) -> &str {
        "inappropriate_intimacy"
    }

    fn description(&self) -> &str {
        "Bidirectional imports between files"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let current = normalize_path(&ctx.file.path.to_string_lossy());
        let mut checked: HashMap<String, Vec<String>> = HashMap::new();
        ctx.model
            .imports
            .iter()
            .filter_map(|imp| {
                let target = resolve_import(&ctx.file.path.to_string_lossy(), &imp.source);
                if target.is_empty() {
                    return None;
                }
                let reverse = checked
                    .entry(target.clone())
                    .or_insert_with(|| read_file_imports(&target));
                let has_cycle = reverse
                    .iter()
                    .any(|ri| normalize_path(&resolve_import(&target, ri)) == current);
                has_cycle.then(|| make_finding(ctx, imp, &current))
            })
            .collect()
    }
}

fn make_finding(ctx: &AnalysisContext, imp: &crate::ImportInfo, current: &str) -> Finding {
    Finding {
        smell_name: "inappropriate_intimacy".into(),
        category: SmellCategory::Couplers,
        severity: Severity::Warning,
        location: Location {
            path: ctx.file.path.clone(),
            start_line: imp.line,
            end_line: imp.line,
            name: None,
        },
        message: format!(
            "Bidirectional dependency between `{}` and `{}`, consider Move Method or Hide Delegate",
            current, imp.source
        ),
        suggested_refactorings: vec!["Move Method".into(), "Hide Delegate".into()],
    }
}

fn normalize_path(p: &str) -> String {
    p.replace('\\', "/").trim_start_matches("./").to_string()
}

fn normalize_import(source: &str) -> String {
    source
        .trim_matches('"')
        .trim_matches('\'')
        .replace('\\', "/")
        .to_string()
}

/// Resolve a relative import path against a base file path.
fn resolve_import(base: &str, import: &str) -> String {
    if !import.starts_with('.') {
        return String::new(); // skip non-relative imports
    }
    let base_dir = std::path::Path::new(base)
        .parent()
        .unwrap_or(std::path::Path::new(""));
    let resolved = base_dir.join(import);
    // Try common extensions
    for ext in &["", ".ts", ".tsx", ".rs"] {
        let with_ext = format!("{}{}", resolved.to_string_lossy(), ext);
        if std::path::Path::new(&with_ext).exists() {
            return normalize_path(&with_ext);
        }
    }
    normalize_path(&resolved.to_string_lossy())
}

/// Read import sources from a file (lightweight grep, no full parse).
fn read_file_imports(path: &str) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            // Match: import ... from "..." or use ...;
            if trimmed.starts_with("import")
                && let Some(from_idx) = trimmed.find("from")
            {
                let rest = trimmed[from_idx + 4..].trim().trim_matches(';');
                return Some(normalize_import(rest));
            }
            None
        })
        .collect()
}
