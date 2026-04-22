use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Check naming conventions for functions and classes.
pub struct NamingAnalyzer {
    pub min_name_length: usize,
    pub max_name_length: usize,
}

impl Default for NamingAnalyzer {
    fn default() -> Self {
        Self {
            min_name_length: 2,
            max_name_length: 50,
        }
    }
}

impl Plugin for NamingAnalyzer {
    fn name(&self) -> &str {
        "naming"
    }

    fn description(&self) -> &str {
        "Naming convention violations"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        self.check_functions(ctx, &mut findings);
        self.check_classes(ctx, &mut findings);
        findings
    }
}

impl NamingAnalyzer {
    fn check_functions(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        for f in &ctx.model.functions {
            let check = NameCheck {
                name: &f.name,
                kind: "Function",
                path: &ctx.file.path,
                start_line: f.start_line,
                start_col: f.name_col,
                end_line: f.start_line,
                end_col: f.name_end_col,
            };
            if let Some(finding) = check_name(&check, self.min_name_length, self.max_name_length) {
                findings.push(finding);
            }
        }
    }

    fn check_classes(&self, ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
        for c in &ctx.model.classes {
            if let Some(f) = check_pascal_case(c, &ctx.file.path) {
                findings.push(f);
            }
            let check = NameCheck {
                name: &c.name,
                kind: "Class",
                path: &ctx.file.path,
                start_line: c.start_line,
                start_col: c.name_col,
                end_line: c.start_line,
                end_col: c.name_end_col,
            };
            if let Some(f) = check_name(&check, self.min_name_length, self.max_name_length) {
                findings.push(f);
            }
        }
    }
}

/// Check if a class name violates PascalCase convention.
fn check_pascal_case(c: &crate::ClassInfo, path: &std::path::Path) -> Option<Finding> {
    if c.name.is_empty() || c.name.chars().next().is_some_and(|ch| ch.is_uppercase()) {
        return None;
    }
    Some(Finding {
        smell_name: "naming_convention".into(),
        category: SmellCategory::Bloaters,
        severity: Severity::Hint,
        location: Location {
            path: path.to_path_buf(),
            start_line: c.start_line,
            start_col: c.name_col,
            end_line: c.start_line,
            end_col: c.name_end_col,
            name: Some(c.name.clone()),
        },
        message: format!("Class `{}` should use PascalCase", c.name),
        suggested_refactorings: vec!["Rename Method".into()],
        ..Default::default()
    })
}

struct NameCheck<'a> {
    name: &'a str,
    kind: &'a str,
    path: &'a std::path::Path,
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
}

fn check_name(check: &NameCheck, min_len: usize, max_len: usize) -> Option<Finding> {
    let (smell, severity, qualifier, limit) = if check.name.len() < min_len {
        ("naming_too_short", Severity::Warning, "short", min_len)
    } else if check.name.len() > max_len {
        ("naming_too_long", Severity::Hint, "long", max_len)
    } else {
        return None;
    };
    let bound_label = if qualifier == "short" { "min" } else { "max" };
    Some(Finding {
        smell_name: smell.into(),
        category: SmellCategory::Bloaters,
        severity,
        location: Location {
            path: check.path.to_path_buf(),
            start_line: check.start_line,
            start_col: check.start_col,
            end_line: check.end_line,
            end_col: check.end_col,
            name: Some(check.name.to_string()),
        },
        message: format!(
            "{} `{}` name is too {} ({} chars, {}: {})",
            check.kind,
            check.name,
            qualifier,
            check.name.len(),
            bound_label,
            limit
        ),
        suggested_refactorings: vec!["Rename Method".into()],
        ..Default::default()
    })
}
