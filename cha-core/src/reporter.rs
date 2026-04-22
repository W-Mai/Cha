use crate::Finding;

/// Output format for analysis results.
pub trait Reporter {
    fn render(&self, findings: &[Finding]) -> String;
}

/// Colored terminal output.
pub struct TerminalReporter {
    /// If true, show all findings without aggregation.
    pub show_all: bool,
}

impl Reporter for TerminalReporter {
    fn render(&self, findings: &[Finding]) -> String {
        if findings.is_empty() {
            return "No issues found.".into();
        }
        let mut out = String::new();
        if self.show_all {
            for f in findings {
                render_terminal_finding(&mut out, f);
            }
        } else {
            render_grouped(&mut out, findings);
        }
        render_summary(&mut out, findings);
        out
    }
}

fn render_grouped(out: &mut String, findings: &[Finding]) {
    let mut groups: std::collections::BTreeMap<&str, Vec<&Finding>> =
        std::collections::BTreeMap::new();
    for f in findings {
        groups.entry(&f.smell_name).or_default().push(f);
    }
    for (name, group) in &groups {
        if group.len() <= 5 {
            for f in group {
                render_terminal_finding(out, f);
            }
        } else {
            render_aggregated(out, name, group);
        }
    }
}

fn render_aggregated(out: &mut String, name: &str, group: &[&Finding]) {
    let mut sorted: Vec<&&Finding> = group.iter().collect();
    sorted.sort_by(|a, b| {
        b.actual_value
            .unwrap_or(0.0)
            .partial_cmp(&a.actual_value.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let icon = severity_icon(&sorted[0].severity);
    out.push_str(&format!("{icon} [{name}] {} occurrences\n", group.len()));
    for f in sorted.iter().take(3) {
        let loc = if f.location.start_col > 0 {
            format!(
                "{}:{}:{}",
                f.location.path.display(),
                f.location.start_line,
                f.location.start_col
            )
        } else {
            format!("{}:{}", f.location.path.display(), f.location.start_line)
        };
        out.push_str(&format!("  → {loc} {}\n", f.message));
    }
    if group.len() > 3 {
        out.push_str(&format!(
            "  … and {} more (use --all to show all)\n",
            group.len() - 3
        ));
    }
}

fn render_summary(out: &mut String, findings: &[Finding]) {
    let errors = findings
        .iter()
        .filter(|f| f.severity == crate::Severity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|f| f.severity == crate::Severity::Warning)
        .count();
    let hints = findings.len() - errors - warnings;
    out.push_str(&format!(
        "\n{} issue(s) found ({} error, {} warning, {} hint).",
        findings.len(),
        errors,
        warnings,
        hints
    ));
}

fn severity_icon(s: &crate::Severity) -> &'static str {
    match s {
        crate::Severity::Error => "✗",
        crate::Severity::Warning => "⚠",
        crate::Severity::Hint => "ℹ",
    }
}

fn render_terminal_finding(out: &mut String, f: &Finding) {
    let loc = if f.location.start_col > 0 {
        format!(
            "{}:{}:{}-{}:{}",
            f.location.path.display(),
            f.location.start_line,
            f.location.start_col,
            f.location.end_line,
            f.location.end_col,
        )
    } else {
        format!(
            "{}:{}-{}",
            f.location.path.display(),
            f.location.start_line,
            f.location.end_line,
        )
    };
    out.push_str(&format!(
        "{} [{}] {loc} {}\n",
        severity_icon(&f.severity),
        f.smell_name,
        f.message,
    ));
    if !f.suggested_refactorings.is_empty() {
        out.push_str(&format!(
            "  → suggested: {}\n",
            f.suggested_refactorings.join(", ")
        ));
    }
}

/// JSON output.
pub struct JsonReporter;

impl Reporter for JsonReporter {
    fn render(&self, findings: &[Finding]) -> String {
        serde_json::to_string_pretty(findings).unwrap_or_default()
    }
}

impl JsonReporter {
    /// Render findings with health scores.
    pub fn render_with_scores(
        &self,
        findings: &[Finding],
        scores: &[crate::health::HealthScore],
    ) -> String {
        let report = serde_json::json!({
            "findings": findings,
            "health_scores": scores.iter().map(|s| serde_json::json!({
                "path": s.path,
                "grade": s.grade.to_string(),
                "debt_minutes": s.debt_minutes,
                "lines": s.lines,
            })).collect::<Vec<_>>(),
        });
        serde_json::to_string_pretty(&report).unwrap_or_default()
    }
}

/// Structured context for LLM-assisted refactoring.
pub struct LlmContextReporter;

impl Reporter for LlmContextReporter {
    fn render(&self, findings: &[Finding]) -> String {
        if findings.is_empty() {
            return "No code smells detected.".into();
        }
        let mut out = String::from("# Code Smell Analysis\n\n");
        for (i, f) in findings.iter().enumerate() {
            render_llm_issue(&mut out, i, f);
        }
        out.push_str("Please apply the suggested refactorings to improve code quality.\n");
        out
    }
}

/// Render a single finding as an LLM-readable markdown section.
fn render_llm_issue(out: &mut String, index: usize, f: &Finding) {
    out.push_str(&format!("## Issue {}\n\n", index + 1));
    out.push_str(&format!("- **Smell**: {}\n", f.smell_name));
    out.push_str(&format!("- **Category**: {:?}\n", f.category));
    out.push_str(&format!("- **Severity**: {:?}\n", f.severity));
    let loc = if f.location.start_col > 0 {
        format!(
            "{}:{}:{}-{}:{}",
            f.location.path.display(),
            f.location.start_line,
            f.location.start_col,
            f.location.end_line,
            f.location.end_col,
        )
    } else {
        format!(
            "{}:{}-{}",
            f.location.path.display(),
            f.location.start_line,
            f.location.end_line,
        )
    };
    out.push_str(&format!("- **Location**: {loc}"));
    if let Some(name) = &f.location.name {
        out.push_str(&format!(" (`{}`)", name));
    }
    out.push('\n');
    out.push_str(&format!("- **Problem**: {}\n", f.message));
    if !f.suggested_refactorings.is_empty() {
        out.push_str("- **Suggested refactorings**:\n");
        for r in &f.suggested_refactorings {
            out.push_str(&format!("  - {}\n", r));
        }
    }
    out.push('\n');
}

/// SARIF output for GitHub Code Scanning.
pub struct SarifReporter;

impl Reporter for SarifReporter {
    fn render(&self, findings: &[Finding]) -> String {
        self.render_with_scores(findings, &[])
    }
}

impl SarifReporter {
    /// Render SARIF with optional health score properties.
    pub fn render_with_scores(
        &self,
        findings: &[Finding],
        scores: &[crate::health::HealthScore],
    ) -> String {
        let rules = build_sarif_rules(findings);
        let results = build_sarif_results(findings);
        let mut run = serde_json::json!({
            "tool": {
                "driver": {
                    "name": "cha",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rules,
                }
            },
            "results": results,
        });
        if !scores.is_empty() {
            run["properties"] = serde_json::json!({
                "health_scores": scores.iter().map(|s| serde_json::json!({
                    "path": s.path,
                    "grade": s.grade.to_string(),
                    "debt_minutes": s.debt_minutes,
                    "lines": s.lines,
                })).collect::<Vec<_>>(),
            });
        }
        let sarif = serde_json::json!({
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
            "version": "2.1.0",
            "runs": [run],
        });
        serde_json::to_string_pretty(&sarif).unwrap_or_default()
    }
}

/// Collect unique rule descriptors from findings.
fn build_sarif_rules(findings: &[Finding]) -> Vec<serde_json::Value> {
    findings
        .iter()
        .map(|f| &f.smell_name)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|name| {
            serde_json::json!({
                "id": name,
                "shortDescription": { "text": name },
            })
        })
        .collect()
}

/// Convert findings into SARIF result entries.
fn build_sarif_results(findings: &[Finding]) -> Vec<serde_json::Value> {
    findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "ruleId": f.smell_name,
                "level": match f.severity {
                    crate::Severity::Error => "error",
                    crate::Severity::Warning => "warning",
                    crate::Severity::Hint => "note",
                },
                "message": { "text": f.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": f.location.path.to_string_lossy(),
                        },
                        "region": {
                            "startLine": f.location.start_line,
                            "startColumn": f.location.start_col + 1,
                            "endLine": f.location.end_line,
                            "endColumn": f.location.end_col + 1,
                        }
                    }
                }]
            })
        })
        .collect()
}
