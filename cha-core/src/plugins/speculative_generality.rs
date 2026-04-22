use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect interfaces/traits with only one implementation (over-abstraction).
pub struct SpeculativeGeneralityAnalyzer;

impl Default for SpeculativeGeneralityAnalyzer {
    fn default() -> Self {
        Self
    }
}

impl Plugin for SpeculativeGeneralityAnalyzer {
    fn name(&self) -> &str {
        "speculative_generality"
    }

    fn description(&self) -> &str {
        "Interface with too few implementations"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        // Count how many classes extend/implement each interface in this file
        let interfaces: Vec<_> = ctx
            .model
            .classes
            .iter()
            .filter(|c| c.is_interface)
            .collect();
        if interfaces.is_empty() {
            return vec![];
        }
        let mut findings = Vec::new();
        for iface in &interfaces {
            let impl_count = ctx
                .model
                .classes
                .iter()
                .filter(|c| c.parent_name.as_deref() == Some(&iface.name))
                .count();
            // Only flag if exactly 0 or 1 implementations in the same file
            if impl_count <= 1 {
                findings.push(Finding {
                    smell_name: "speculative_generality".into(),
                    category: SmellCategory::Dispensables,
                    severity: Severity::Hint,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: iface.start_line,
                        end_line: iface.end_line,
                        name: Some(iface.name.clone()),
                        ..Default::default()
                    },
                    message: format!(
                        "Interface `{}` has only {} implementation(s) in this file, consider Collapse Hierarchy",
                        iface.name, impl_count
                    ),
                    suggested_refactorings: vec![
                        "Collapse Hierarchy".into(),
                        "Inline Class".into(),
                    ],
                    ..Default::default()
                });
            }
        }
        findings
    }
}
