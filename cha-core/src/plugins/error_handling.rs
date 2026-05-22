use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect error handling smells:
/// - Empty catch/except blocks (silently swallowed errors)
/// - Excessive unwrap()/expect() calls in Rust
///
/// Both checks use tree-sitter queries when AST is available, so substring
/// matches inside strings or comments don't trigger false positives.
///
/// ## References
///
/// [1] G. Padua and W. Shang, "Revisiting Exception Handling Practices
///     with Exception Flow Analysis," Empirical Software Engineering,
///     vol. 23, no. 6, pp. 3337–3383, 2018.
///     doi: 10.1007/s10664-018-9601-8.
///
/// [2] A. Rahman, C. Parnin, and L. Williams, "The Seven Sins: Security
///     Smells in Infrastructure as Code Scripts," in Proc. 41st Int. Conf.
///     Software Engineering (ICSE), 2019, pp. 164–175.
///     doi: 10.1109/ICSE.2019.00033.
pub struct ErrorHandlingAnalyzer {
    pub max_unwraps_per_function: usize,
}

impl Default for ErrorHandlingAnalyzer {
    fn default() -> Self {
        Self {
            max_unwraps_per_function: 3,
        }
    }
}

impl Plugin for ErrorHandlingAnalyzer {
    fn name(&self) -> &str {
        "error_handling"
    }

    fn smells(&self) -> Vec<String> {
        vec!["empty_catch".into(), "unwrap_abuse".into()]
    }

    fn description(&self) -> &str {
        "Empty catch blocks, unwrap/expect abuse"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let unwrap_sites = collect_unwrap_sites(ctx);
        for f in &ctx.model.functions {
            let in_fn: Vec<&UnwrapSite> = unwrap_sites
                .iter()
                .filter(|s| s.line >= f.start_line && s.line <= f.end_line)
                .collect();
            if in_fn.len() > self.max_unwraps_per_function {
                for site in &in_fn {
                    findings.push(build_unwrap_finding(
                        ctx,
                        f,
                        site,
                        in_fn.len(),
                        self.max_unwraps_per_function,
                    ));
                }
            }
        }
        detect_empty_catch(ctx, &mut findings);
        findings
    }
}

/// A single `.unwrap()` / `.expect(...)` call site.
struct UnwrapSite {
    line: usize,
    start_col: usize,
    end_col: usize,
    matched: String,
}

fn collect_unwrap_sites(ctx: &AnalysisContext) -> Vec<UnwrapSite> {
    if let (Some(tree), Some(lang)) = (ctx.tree, ctx.ts_language) {
        if ctx.model.language != "rust" {
            // unwrap_abuse only meaningful in Rust. Python `.get(default)`,
            // JS optional chaining etc are separate detections.
            return Vec::new();
        }
        let source = ctx.file.content.as_bytes();
        let pattern = r#"(call_expression function: (field_expression field: (field_identifier) @m (#match? @m "^(unwrap|expect)$"))) @site"#;
        let mut out = Vec::new();
        for matches in crate::query::run_query(tree, lang, source, pattern) {
            let Some(site_cap) = matches.iter().find(|c| c.capture_name == "site") else {
                continue;
            };
            let m_cap = matches.iter().find(|c| c.capture_name == "m");
            let matched = match m_cap {
                Some(c) if c.text == "expect" => "expect",
                _ => "unwrap",
            }
            .to_string();
            out.push(UnwrapSite {
                line: site_cap.start_line as usize,
                start_col: site_cap.start_col as usize,
                end_col: site_cap.end_col as usize,
                matched,
            });
        }
        return out;
    }
    // Tree unavailable — legacy substring fallback for unit tests that build
    // SourceModel without a real parse.
    collect_unwrap_sites_legacy(&ctx.file.content)
}

fn collect_unwrap_sites_legacy(content: &str) -> Vec<UnwrapSite> {
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        push_legacy_match(&mut out, i + 1, line, ".unwrap()", "unwrap");
        push_legacy_match(&mut out, i + 1, line, ".expect(", "expect");
    }
    out
}

fn push_legacy_match(
    sites: &mut Vec<UnwrapSite>,
    line: usize,
    text: &str,
    needle: &str,
    matched: &str,
) {
    let mut search_from = 0;
    while let Some(pos) = text[search_from..].find(needle) {
        let abs = search_from + pos;
        sites.push(UnwrapSite {
            line,
            start_col: abs,
            end_col: abs + needle.len(),
            matched: matched.to_string(),
        });
        search_from = abs + needle.len();
    }
}

fn build_unwrap_finding(
    ctx: &AnalysisContext,
    f: &crate::FunctionInfo,
    site: &UnwrapSite,
    total: usize,
    threshold: usize,
) -> Finding {
    Finding {
        smell_name: "unwrap_abuse".into(),
        category: SmellCategory::Security,
        severity: Severity::Warning,
        location: Location {
            path: ctx.file.path.clone(),
            start_line: site.line,
            start_col: site.start_col,
            end_line: site.line,
            end_col: site.end_col,
            name: Some(f.name.clone()),
        },
        message: format!(
            "`.{}()` in `{}` (function has {total} unwrap/expect calls, threshold: {threshold})",
            site.matched, f.name
        ),
        suggested_refactorings: vec!["Use ? operator".into(), "Handle errors explicitly".into()],
        ..Default::default()
    }
}

/// Rust has no `catch` syntax — skip. Substring "catch" in a Rust string or
/// comment used to false-positive.
fn detect_empty_catch(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    let (Some(tree), Some(lang)) = (ctx.tree, ctx.ts_language) else {
        return;
    };
    let source = ctx.file.content.as_bytes();
    let patterns: &[&str] = match ctx.model.language.as_str() {
        // TS/JS: catch_clause body is a statement_block; flag if no statements.
        "typescript" => &["(catch_clause body: (statement_block) @body) @site"],
        // Python: except_clause body is a block; flag if it's just `pass`.
        "python" => &["(except_clause body: (block) @body) @site"],
        _ => return,
    };
    for pat in patterns {
        for matches in crate::query::run_query(tree, lang, source, pat) {
            let Some(body) = matches.iter().find(|c| c.capture_name == "body") else {
                continue;
            };
            let Some(site) = matches.iter().find(|c| c.capture_name == "site") else {
                continue;
            };
            if !is_body_empty(body) {
                continue;
            }
            findings.push(Finding {
                smell_name: "empty_catch".into(),
                category: SmellCategory::Security,
                severity: Severity::Warning,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: site.start_line as usize,
                    start_col: site.start_col as usize,
                    end_line: site.end_line as usize,
                    end_col: site.end_col as usize,
                    name: None,
                },
                message: "Empty catch/except block — errors are silently swallowed".into(),
                suggested_refactorings: vec![
                    "Log the error".into(),
                    "Re-throw or handle explicitly".into(),
                ],
                ..Default::default()
            });
        }
    }
}

fn is_body_empty(body: &crate::query::QueryMatch) -> bool {
    let trimmed = body.text.trim();
    trimmed == "{}" || trimmed == "pass" || trimmed.is_empty()
}
