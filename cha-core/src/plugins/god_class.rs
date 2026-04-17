use std::collections::HashSet;

use crate::{
    AnalysisContext, ClassInfo, Finding, FunctionInfo, Location, Plugin, Severity, SmellCategory,
};

/// Detect God Classes using the detection strategy from [1]:
///
///   (ATFD > Few) AND (WMC >= VeryHigh) AND (TCC < 1/3)
///
/// ## References
///
/// [1] M. Lanza and R. Marinescu, "Object-Oriented Metrics in Practice:
///     Using Software Metrics to Characterize, Evaluate, and Improve the
///     Design of Object-Oriented Systems," Springer, 2006.
///     doi: 10.1007/3-540-39538-5. Chapter 6.1, pp. 79–83.
///     Thresholds derived from Table A.2 (45 Java projects).
pub struct GodClassAnalyzer {
    /// ATFD threshold: Access to Foreign Data (Few = 5)
    pub max_external_refs: usize,
    /// WMC threshold: Weighted Method Count (VeryHigh = 47)
    pub min_wmc: usize,
}

impl Default for GodClassAnalyzer {
    fn default() -> Self {
        Self {
            max_external_refs: 5,
            min_wmc: 47,
        }
    }
}

impl Plugin for GodClassAnalyzer {
    fn name(&self) -> &str {
        "god_class"
    }

    fn description(&self) -> &str {
        "God Class: high coupling, low cohesion"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model
            .classes
            .iter()
            .filter_map(|c| self.check(c, ctx))
            .collect()
    }
}

impl GodClassAnalyzer {
    fn check(&self, c: &ClassInfo, ctx: &AnalysisContext) -> Option<Finding> {
        let methods = class_methods(c, ctx);
        if methods.is_empty() {
            return None;
        }
        let atfd = count_atfd(&methods);
        let wmc: usize = methods.iter().map(|f| f.complexity).sum();
        let tcc = compute_tcc(&methods);
        if atfd <= self.max_external_refs || wmc < self.min_wmc || tcc >= 0.33 {
            return None;
        }
        Some(Finding {
            smell_name: "god_class".into(),
            category: SmellCategory::Bloaters,
            severity: Severity::Warning,
            location: Location {
                path: ctx.file.path.clone(),
                start_line: c.start_line,
                end_line: c.end_line,
                name: Some(c.name.clone()),
            },
            message: format!(
                "Class `{}` is a God Class (ATFD={atfd}, WMC={wmc}, TCC={tcc:.2})",
                c.name
            ),
            suggested_refactorings: vec![
                "Extract Class".into(),
                "Single Responsibility Principle".into(),
            ],
            ..Default::default()
        })
    }
}

fn class_methods<'a>(c: &ClassInfo, ctx: &'a AnalysisContext) -> Vec<&'a FunctionInfo> {
    ctx.model
        .functions
        .iter()
        .filter(|f| f.start_line >= c.start_line && f.end_line <= c.end_line)
        .collect()
}

fn count_atfd(methods: &[&FunctionInfo]) -> usize {
    let ext: HashSet<&str> = methods
        .iter()
        .flat_map(|f| f.external_refs.iter().map(|s| s.as_str()))
        .collect();
    ext.len()
}

/// Tight Class Cohesion: ratio of method pairs sharing at least one field.
fn compute_tcc(methods: &[&FunctionInfo]) -> f64 {
    let sets: Vec<HashSet<&str>> = methods
        .iter()
        .map(|f| f.referenced_fields.iter().map(|s| s.as_str()).collect())
        .filter(|s: &HashSet<&str>| !s.is_empty())
        .collect();
    if sets.len() < 2 {
        return 1.0;
    }
    let mut total = 0usize;
    let mut shared = 0usize;
    for i in 0..sets.len() {
        for j in (i + 1)..sets.len() {
            total += 1;
            if sets[i].intersection(&sets[j]).next().is_some() {
                shared += 1;
            }
        }
    }
    if total == 0 {
        1.0
    } else {
        shared as f64 / total as f64
    }
}
