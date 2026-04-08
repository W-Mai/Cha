use crate::Finding;

/// Output format for analysis results.
pub trait Reporter {
    fn render(&self, findings: &[Finding]) -> String;
}

/// Colored terminal output.
pub struct TerminalReporter;

impl Reporter for TerminalReporter {
    fn render(&self, findings: &[Finding]) -> String {
        if findings.is_empty() {
            return "No issues found.".into();
        }
        let mut out = String::new();
        for f in findings {
            let icon = match f.severity {
                crate::Severity::Error => "✗",
                crate::Severity::Warning => "⚠",
                crate::Severity::Hint => "ℹ",
            };
            out.push_str(&format!(
                "{} [{}] {}:{}-{} {}\n",
                icon,
                f.smell_name,
                f.location.path.display(),
                f.location.start_line,
                f.location.end_line,
                f.message,
            ));
            if !f.suggested_refactorings.is_empty() {
                out.push_str(&format!(
                    "  → suggested: {}\n",
                    f.suggested_refactorings.join(", ")
                ));
            }
        }
        let errors = findings
            .iter()
            .filter(|f| f.severity == crate::Severity::Error)
            .count();
        let warnings = findings
            .iter()
            .filter(|f| f.severity == crate::Severity::Warning)
            .count();
        let hints = findings
            .iter()
            .filter(|f| f.severity == crate::Severity::Hint)
            .count();
        out.push_str(&format!(
            "\n{} issue(s) found ({} error, {} warning, {} hint).",
            findings.len(),
            errors,
            warnings,
            hints
        ));
        out
    }
}

/// JSON output.
pub struct JsonReporter;

impl Reporter for JsonReporter {
    fn render(&self, findings: &[Finding]) -> String {
        serde_json::to_string_pretty(findings).unwrap_or_default()
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
    out.push_str(&format!(
        "- **Location**: {}:{}-{}",
        f.location.path.display(),
        f.location.start_line,
        f.location.end_line,
    ));
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
        let rules = build_sarif_rules(findings);
        let results = build_sarif_results(findings);
        let sarif = serde_json::json!({
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "cha",
                        "version": env!("CARGO_PKG_VERSION"),
                        "rules": rules,
                    }
                },
                "results": results,
            }]
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
                            "endLine": f.location.end_line,
                        }
                    }
                }]
            })
        })
        .collect()
}
