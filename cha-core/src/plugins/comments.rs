use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect functions where comment lines exceed a threshold ratio.
pub struct CommentsAnalyzer {
    pub max_comment_ratio: f64,
    pub min_lines: usize,
}

impl Default for CommentsAnalyzer {
    fn default() -> Self {
        Self {
            max_comment_ratio: 0.3,
            min_lines: 10,
        }
    }
}

impl Plugin for CommentsAnalyzer {
    fn name(&self) -> &str {
        "comments"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .functions
            .iter()
            .filter_map(|f| {
                if f.line_count < self.min_lines || f.comment_lines == 0 {
                    return None;
                }
                let ratio = f.comment_lines as f64 / f.line_count as f64;
                if ratio <= self.max_comment_ratio {
                    return None;
                }
                Some(Finding {
                    smell_name: "excessive_comments".into(),
                    category: SmellCategory::Dispensables,
                    severity: Severity::Hint,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: f.start_line,
                        end_line: f.end_line,
                        name: Some(f.name.clone()),
                    },
                    message: format!(
                        "Function `{}` has {:.0}% comment lines ({}/{}), consider Extract Method",
                        f.name,
                        ratio * 100.0,
                        f.comment_lines,
                        f.line_count
                    ),
                    suggested_refactorings: vec!["Extract Method".into(), "Rename Method".into()],
                })
            })
            .collect()
    }
}
