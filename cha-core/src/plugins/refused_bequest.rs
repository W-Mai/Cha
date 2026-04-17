use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect classes that inherit but override most parent methods (refused bequest).
pub struct RefusedBequestAnalyzer {
    pub min_override_ratio: f64,
    pub min_methods: usize,
}

impl Default for RefusedBequestAnalyzer {
    fn default() -> Self {
        Self {
            min_override_ratio: 0.5,
            min_methods: 3,
        }
    }
}

impl Plugin for RefusedBequestAnalyzer {
    fn name(&self) -> &str {
        "refused_bequest"
    }

    fn description(&self) -> &str {
        "Subclass overrides most parent methods"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .classes
            .iter()
            .filter_map(|c| {
                let parent = c.parent_name.as_ref()?;
                if c.method_count < self.min_methods || c.override_count == 0 {
                    return None;
                }
                let ratio = c.override_count as f64 / c.method_count as f64;
                if ratio < self.min_override_ratio {
                    return None;
                }
                Some(Finding {
                    smell_name: "refused_bequest".into(),
                    category: SmellCategory::OoAbusers,
                    severity: Severity::Hint,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: c.start_line,
                        end_line: c.end_line,
                        name: Some(c.name.clone()),
                    },
                    message: format!(
                        "Class `{}` overrides {}/{} methods from `{}`, consider Replace Inheritance with Delegation",
                        c.name, c.override_count, c.method_count, parent
                    ),
                    suggested_refactorings: vec![
                        "Replace Inheritance with Delegation".into(),
                        "Push Down Method".into(),
                    ],
                    actual_value: Some(ratio),
                    threshold: Some(self.min_override_ratio),
                })
            })
            .collect()
    }
}
