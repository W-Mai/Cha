use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Configurable thresholds for length checks.
pub struct LengthAnalyzer {
    pub max_function_lines: usize,
    pub max_class_methods: usize,
    pub max_class_lines: usize,
    pub max_file_lines: usize,
}

impl Default for LengthAnalyzer {
    fn default() -> Self {
        Self {
            max_function_lines: 50,
            max_class_methods: 10,
            max_class_lines: 200,
            max_file_lines: 500,
        }
    }
}

impl Plugin for LengthAnalyzer {
    fn name(&self) -> &str {
        "length"
    }

    fn description(&self) -> &str {
        "Long method, large class, or large file"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        self.check_functions(ctx, &mut findings);
        self.check_classes(ctx, &mut findings);
        self.check_file(ctx, &mut findings);
        findings
    }
}

impl LengthAnalyzer {
    fn check_functions(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        let complexity_threshold = 10.0_f64; // default warning threshold
        for f in &ctx.model.functions {
            let line_ratio = f.line_count as f64 / self.max_function_lines as f64;
            let complexity_factor = (f.complexity as f64 / complexity_threshold).max(1.0);
            let risk = line_ratio * complexity_factor;
            if risk < 1.0 {
                continue;
            }
            let severity = risk_severity(risk);
            findings.push(Finding {
                smell_name: "long_method".into(),
                category: SmellCategory::Bloaters,
                severity,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: f.start_line,
                    end_line: f.end_line,
                    name: Some(f.name.clone()),
                },
                message: format!(
                    "Function `{}` is {} lines (threshold: {}, risk: {:.1})",
                    f.name, f.line_count, self.max_function_lines, risk
                ),
                suggested_refactorings: vec!["Extract Method".into()],
                actual_value: Some(risk),
                threshold: Some(1.0),
            });
        }
    }

    fn check_classes(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        for c in &ctx.model.classes {
            if let Some(f) = self.check_single_class(ctx, c) {
                findings.push(f);
            }
        }
    }

    /// Build a finding for a single class if it exceeds size thresholds.
    fn check_single_class(&self, ctx: &AnalysisContext, c: &crate::ClassInfo) -> Option<Finding> {
        let over_methods = c.method_count > self.max_class_methods;
        let over_lines = c.line_count > self.max_class_lines;
        if !over_methods && !over_lines {
            return None;
        }
        let mut reasons = Vec::new();
        if over_methods {
            reasons.push(format!("{} methods", c.method_count));
        }
        if over_lines {
            reasons.push(format!("{} lines", c.line_count));
        }
        Some(Finding {
            smell_name: "large_class".into(),
            category: SmellCategory::Bloaters,
            severity: if over_methods && over_lines {
                Severity::Error
            } else {
                Severity::Warning
            },
            location: Location {
                path: ctx.file.path.clone(),
                start_line: c.start_line,
                end_line: c.end_line,
                name: Some(c.name.clone()),
            },
            message: format!("Class `{}` is too large ({})", c.name, reasons.join(", ")),
            suggested_refactorings: vec!["Extract Class".into()],
            actual_value: Some(c.line_count as f64),
            threshold: Some(self.max_class_lines as f64),
        })
    }

    fn check_file(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        if ctx.model.total_lines > self.max_file_lines {
            findings.push(Finding {
                smell_name: "large_file".into(),
                category: SmellCategory::Bloaters,
                severity: severity_for_ratio(ctx.model.total_lines, self.max_file_lines),
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: 1,
                    end_line: ctx.model.total_lines,
                    name: None,
                },
                message: format!(
                    "File is {} lines (threshold: {})",
                    ctx.model.total_lines, self.max_file_lines
                ),
                suggested_refactorings: vec!["Extract Class".into(), "Move Method".into()],
                actual_value: Some(ctx.model.total_lines as f64),
                threshold: Some(self.max_file_lines as f64),
            });
        }
    }
}

fn severity_for_ratio(actual: usize, threshold: usize) -> Severity {
    let ratio = actual as f64 / threshold as f64;
    if ratio > 2.0 {
        Severity::Error
    } else {
        Severity::Warning
    }
}

fn risk_severity(risk: f64) -> Severity {
    if risk >= 4.0 {
        Severity::Error
    } else if risk >= 2.0 {
        Severity::Warning
    } else {
        Severity::Hint
    }
}
