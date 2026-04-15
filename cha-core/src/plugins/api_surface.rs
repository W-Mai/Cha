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

    fn description(&self) -> &str {
        "Exported ratio too high, narrow the public API"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let total = ctx.model.functions.len() + ctx.model.classes.len();
        if total < 5 {
            return vec![];
        }

        let exported = count_exported(ctx);
        let ratio = exported as f64 / total as f64;

        if exported > self.max_exported_count || ratio > self.max_exported_ratio {
            vec![self.make_finding(ctx, exported, total, ratio)]
        } else {
            vec![]
        }
    }
}

/// Count total exported functions and classes.
fn count_exported(ctx: &AnalysisContext) -> usize {
    let fns = ctx.model.functions.iter().filter(|f| f.is_exported).count();
    let cls = ctx.model.classes.iter().filter(|c| c.is_exported).count();
    fns + cls
}

impl ApiSurfaceAnalyzer {
    /// Build the large API surface finding.
    fn make_finding(
        &self,
        ctx: &AnalysisContext,
        exported: usize,
        total: usize,
        ratio: f64,
    ) -> Finding {
        Finding {
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
        }
    }
}
