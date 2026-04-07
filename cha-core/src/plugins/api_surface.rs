use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Analyze the ratio of exported (public) API surface.
pub struct ApiSurfaceAnalyzer {
    pub max_exported_ratio: f64,
    pub max_exported_count: usize,
}

impl Default for ApiSurfaceAnalyzer {
    fn default() -> Self {
        Self {
            max_exported_ratio: 0.8,
            max_exported_count: 20,
        }
    }
}

impl Plugin for ApiSurfaceAnalyzer {
    fn name(&self) -> &str {
        "api_surface"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        let total_fns = ctx.model.functions.len();
        let total_classes = ctx.model.classes.len();
        let total = total_fns + total_classes;
        if total == 0 {
            return findings;
        }

        let exported_fns = ctx.model.functions.iter().filter(|f| f.is_exported).count();
        let exported_classes = ctx.model.classes.iter().filter(|c| c.is_exported).count();
        let exported = exported_fns + exported_classes;

        let ratio = exported as f64 / total as f64;

        if exported > self.max_exported_count || ratio > self.max_exported_ratio {
            findings.push(Finding {
                smell_name: "large_api_surface".into(),
                category: SmellCategory::Bloaters,
                severity: Severity::Warning,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: 1,
                    end_line: ctx.model.total_lines,
                    name: None,
                },
                message: format!(
                    "File exports {}/{} items ({:.0}%), consider narrowing the public API",
                    exported,
                    total,
                    ratio * 100.0
                ),
                suggested_refactorings: vec!["Hide Method".into(), "Extract Class".into()],
            });
        }

        findings
    }
}
