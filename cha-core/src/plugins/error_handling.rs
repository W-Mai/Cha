use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect error handling smells:
/// - Empty catch/except blocks (silently swallowed errors)
/// - Excessive unwrap()/expect() calls in Rust
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
        let lines: Vec<&str> = ctx.file.content.lines().collect();

        for f in &ctx.model.functions {
            let sites = collect_unwrap_sites(&lines, f.start_line, f.end_line);
            if sites.len() > self.max_unwraps_per_function {
                for site in &sites {
                    findings.push(build_unwrap_finding(
                        ctx,
                        f,
                        site,
                        sites.len(),
                        self.max_unwraps_per_function,
                    ));
                }
            }
        }

        detect_empty_catch(&lines, ctx, &mut findings);
        findings
    }
}

/// A single `.unwrap()` / `.expect(` call site inside a function body.
struct UnwrapSite {
    /// 1-based line number in the source file.
    line: usize,
    /// 0-based column of the start of the matching substring (`.unwrap()` or `.expect(`).
    start_col: usize,
    /// 0-based column of the end of the matching substring.
    end_col: usize,
    /// Raw matched substring, used in the message for clarity.
    matched: &'static str,
}

fn collect_unwrap_sites(lines: &[&str], start: usize, end: usize) -> Vec<UnwrapSite> {
    let mut sites = Vec::new();
    for (idx, line) in lines
        .iter()
        .enumerate()
        .take(end.min(lines.len()))
        .skip(start.saturating_sub(1))
    {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        push_matches(&mut sites, idx + 1, line, ".unwrap()");
        push_matches(&mut sites, idx + 1, line, ".expect(");
    }
    sites
}

fn push_matches(sites: &mut Vec<UnwrapSite>, line: usize, text: &str, needle: &'static str) {
    let mut search_from = 0;
    while let Some(pos) = text[search_from..].find(needle) {
        let abs = search_from + pos;
        sites.push(UnwrapSite {
            line,
            start_col: abs,
            end_col: abs + needle.len(),
            matched: needle,
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
            "`{}` in `{}` (function has {total} unwrap/expect calls, threshold: {threshold})",
            site.matched, f.name
        ),
        suggested_refactorings: vec!["Use ? operator".into(), "Handle errors explicitly".into()],
        ..Default::default()
    }
}

fn detect_empty_catch(lines: &[&str], ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let is_catch = trimmed.starts_with("catch")
            || trimmed.starts_with("except")
            || trimmed.starts_with("} catch")
            || trimmed.starts_with("rescue");
        if !is_catch {
            continue;
        }
        if let Some(next) = lines.iter().skip(i + 1).find(|l| !l.trim().is_empty()) {
            let next_trimmed = next.trim();
            if next_trimmed == "}" || next_trimmed == "pass" || next_trimmed.is_empty() {
                let col = line.find(trimmed).unwrap_or(0);
                findings.push(Finding {
                    smell_name: "empty_catch".into(),
                    category: SmellCategory::Security,
                    severity: Severity::Warning,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: i + 1,
                        start_col: col,
                        end_line: i + 2,
                        name: None,
                        ..Default::default()
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
}
