use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect fields that are only used in a small fraction of methods.
pub struct TemporaryFieldAnalyzer {
    pub min_methods: usize,
    pub max_usage_ratio: f64,
}

impl Default for TemporaryFieldAnalyzer {
    fn default() -> Self {
        Self {
            min_methods: 3,
            max_usage_ratio: 0.3,
        }
    }
}

impl Plugin for TemporaryFieldAnalyzer {
    fn name(&self) -> &str {
        "temporary_field"
    }

    fn description(&self) -> &str {
        "Fields used in too few methods"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        for class in &ctx.model.classes {
            if class.field_names.is_empty() || class.method_count < self.min_methods {
                continue;
            }
            let methods: Vec<_> = ctx
                .model
                .functions
                .iter()
                .filter(|f| f.start_line >= class.start_line && f.end_line <= class.end_line)
                .collect();
            if methods.len() < self.min_methods {
                continue;
            }
            self.check_fields(ctx, class, &methods, &mut findings);
        }
        findings
    }
}

impl TemporaryFieldAnalyzer {
    fn check_fields(
        &self,
        ctx: &AnalysisContext,
        class: &crate::ClassInfo,
        methods: &[&crate::FunctionInfo],
        findings: &mut Vec<Finding>,
    ) {
        for field in &class.field_names {
            let usage = methods
                .iter()
                .filter(|m| m.referenced_fields.contains(field))
                .count();
            let ratio = usage as f64 / methods.len() as f64;
            if ratio > 0.0 && ratio <= self.max_usage_ratio {
                findings.push(Finding {
                    smell_name: "temporary_field".into(),
                    category: SmellCategory::OoAbusers,
                    severity: Severity::Hint,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: class.start_line,
                        start_col: class.name_col,
                        end_line: class.start_line,
                        end_col: class.name_end_col,
                        name: Some(format!("{}.{}", class.name, field)),
                    },
                    message: format!(
                        "Field `{}` in `{}` is only used in {}/{} methods, consider Extract Class",
                        field,
                        class.name,
                        usage,
                        methods.len()
                    ),
                    suggested_refactorings: vec!["Extract Class".into()],
                    actual_value: Some(ratio),
                    threshold: Some(self.max_usage_ratio),
                });
            }
        }
    }
}
