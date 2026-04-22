use std::collections::HashMap;

use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect functions with identical AST structure (duplicate code).
pub struct DuplicateCodeAnalyzer;

impl Plugin for DuplicateCodeAnalyzer {
    fn name(&self) -> &str {
        "duplicate_code"
    }

    fn description(&self) -> &str {
        "Duplicate code blocks (AST hash)"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let hash_map = build_hash_groups(&ctx.model.functions);
        hash_map
            .values()
            .filter(|g| g.len() >= 2)
            .flat_map(|group| build_duplicate_findings(ctx, group))
            .collect()
    }
}

/// Group non-trivial functions by their body hash.
fn build_hash_groups(functions: &[crate::FunctionInfo]) -> HashMap<u64, Vec<&crate::FunctionInfo>> {
    let mut map: HashMap<u64, Vec<&crate::FunctionInfo>> = HashMap::new();
    for f in functions {
        if let Some(hash) = f.body_hash
            && f.line_count > 10
        {
            map.entry(hash).or_default().push(f);
        }
    }
    map
}

/// Build findings for a group of structurally duplicate functions.
fn build_duplicate_findings(ctx: &AnalysisContext, group: &[&crate::FunctionInfo]) -> Vec<Finding> {
    let names: Vec<&str> = group.iter().map(|f| f.name.as_str()).collect();
    group
        .iter()
        .map(|f| {
            let peers = names
                .iter()
                .filter(|n| **n != f.name)
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            Finding {
                smell_name: "duplicate_code".into(),
                category: SmellCategory::Dispensables,
                severity: Severity::Warning,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: f.start_line,
                    start_col: f.name_col,
                    end_line: f.start_line,
                    end_col: f.name_end_col,
                    name: Some(f.name.clone()),
                },
                message: format!(
                    "Function `{}` has duplicate structure with: {}",
                    f.name, peers
                ),
                suggested_refactorings: vec![
                    "Extract Method".into(),
                    "Form Template Method".into(),
                ],
                ..Default::default()
            }
        })
        .collect()
}
