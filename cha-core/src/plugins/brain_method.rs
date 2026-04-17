use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect Brain Methods using the detection strategy from [1]:
///
///   (LOC > High/2) AND (CYCLO >= High) AND (MAXNESTING >= Several) AND (NOAV > Many)
///
/// Since cha does not track MAXNESTING, we use a three-metric variant:
///
///   (LOC > 65) AND (CYCLO >= 4) AND (NOAV > 7)
///
/// ## References
///
/// [1] M. Lanza and R. Marinescu, "Object-Oriented Metrics in Practice:
///     Using Software Metrics to Characterize, Evaluate, and Improve the
///     Design of Object-Oriented Systems," Springer, 2006.
///     doi: 10.1007/3-540-39538-5. Chapter 6.2.
///     Thresholds derived from Table A.2 (45 Java projects):
///     LOC High = 130, CYCLO High ≈ 3.1, Several = 3, Many = 7–8.
pub struct BrainMethodAnalyzer {
    /// LOC threshold (High/2 = 65)
    pub min_lines: usize,
    /// CYCLO threshold (High ≈ 4)
    pub min_complexity: usize,
    /// NOAV threshold (Many = 7)
    pub min_external_refs: usize,
}

impl Default for BrainMethodAnalyzer {
    fn default() -> Self {
        Self {
            min_lines: 65,
            min_complexity: 4,
            min_external_refs: 7,
        }
    }
}

impl Plugin for BrainMethodAnalyzer {
    fn name(&self) -> &str {
        "brain_method"
    }

    fn description(&self) -> &str {
        "Brain Method: too long, complex, and coupled"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .functions
            .iter()
            .filter(|f| {
                f.line_count >= self.min_lines
                    && f.complexity >= self.min_complexity
                    && f.external_refs.len() >= self.min_external_refs
            })
            .map(|f| Finding {
                smell_name: "brain_method".into(),
                category: SmellCategory::Bloaters,
                severity: Severity::Warning,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: f.start_line,
                    end_line: f.end_line,
                    name: Some(f.name.clone()),
                },
                message: format!(
                    "Function `{}` is a Brain Method ({}L, complexity {}, {} external refs)",
                    f.name,
                    f.line_count,
                    f.complexity,
                    f.external_refs.len()
                ),
                suggested_refactorings: vec!["Extract Method".into(), "Move Method".into()],
                ..Default::default()
            })
            .collect()
    }
}
