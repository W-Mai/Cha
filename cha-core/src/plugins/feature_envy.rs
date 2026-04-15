use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect methods that reference external objects more than their own.
pub struct FeatureEnvyAnalyzer {
    pub min_refs: usize,
    pub external_ratio: f64,
}

impl Default for FeatureEnvyAnalyzer {
    fn default() -> Self {
        Self {
            min_refs: 3,
            external_ratio: 0.7,
        }
    }
}

impl Plugin for FeatureEnvyAnalyzer {
    fn name(&self) -> &str {
        "feature_envy"
    }

    fn description(&self) -> &str {
        "Method uses external data more than its own"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .functions
            .iter()
            .filter(|f| {
                let total = f.external_refs.len();
                total >= self.min_refs
            })
            .map(|f| {
                // Find the most-referenced external object
                let mut counts: std::collections::HashMap<&str, usize> =
                    std::collections::HashMap::new();
                for r in &f.external_refs {
                    *counts.entry(r.as_str()).or_default() += 1;
                }
                let top = counts.values().max().copied().unwrap_or(0);
                let total = f.external_refs.len();
                (f, top, total)
            })
            .filter(|(_, top, total)| (*top as f64 / *total as f64) >= self.external_ratio)
            .map(|(f, _, _)| Finding {
                smell_name: "feature_envy".into(),
                category: SmellCategory::Couplers,
                severity: Severity::Hint,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: f.start_line,
                    end_line: f.end_line,
                    name: Some(f.name.clone()),
                },
                message: format!(
                    "Function `{}` references external objects more than its own data",
                    f.name
                ),
                suggested_refactorings: vec!["Move Method".into()],
            })
            .collect()
    }
}
