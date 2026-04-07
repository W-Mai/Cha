use std::collections::HashMap;

use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect functions with identical AST structure (duplicate code).
pub struct DuplicateCodeAnalyzer;

impl Plugin for DuplicateCodeAnalyzer {
    fn name(&self) -> &str {
        "duplicate_code"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut hash_map: HashMap<u64, Vec<&crate::FunctionInfo>> = HashMap::new();

        for f in &ctx.model.functions {
            if let Some(hash) = f.body_hash {
                // Only consider non-trivial functions (>3 lines)
                if f.line_count > 3 {
                    hash_map.entry(hash).or_default().push(f);
                }
            }
        }

        let mut findings = Vec::new();
        for group in hash_map.values() {
            if group.len() >= 2 {
                let names: Vec<&str> = group.iter().map(|f| f.name.as_str()).collect();
                for f in group {
                    findings.push(Finding {
                        smell_name: "duplicate_code".into(),
                        category: SmellCategory::Dispensables,
                        severity: Severity::Warning,
                        location: Location {
                            path: ctx.file.path.clone(),
                            start_line: f.start_line,
                            end_line: f.end_line,
                            name: Some(f.name.clone()),
                        },
                        message: format!(
                            "Function `{}` has duplicate structure with: {}",
                            f.name,
                            names
                                .iter()
                                .filter(|n| **n != f.name)
                                .copied()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        suggested_refactorings: vec![
                            "Extract Method".into(),
                            "Form Template Method".into(),
                        ],
                    });
                }
            }
        }

        findings
    }
}
