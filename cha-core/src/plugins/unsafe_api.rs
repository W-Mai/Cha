use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect usage of potentially dangerous functions and constructs.
///
/// ## References
///
/// [1] CWE-676: Use of Potentially Dangerous Function.
///     https://cwe.mitre.org/data/definitions/676.html
pub struct UnsafeApiAnalyzer;

impl Plugin for UnsafeApiAnalyzer {
    fn name(&self) -> &str {
        "unsafe_api"
    }

    fn smells(&self) -> Vec<&'static str> {
        vec!["unsafe_api"]
    }

    fn description(&self) -> &str {
        "Dangerous function calls (eval/exec/system)"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let lang = &ctx.model.language;
        let patterns = patterns_for(lang);
        if patterns.is_empty() {
            return vec![];
        }
        let mut findings = Vec::new();
        for (i, line) in ctx.file.content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }
            for &(pat, msg) in &patterns {
                if line.contains(pat) && !is_in_string(line, pat) {
                    let col = line.find(pat).unwrap_or(0);
                    findings.push(Finding {
                        smell_name: "unsafe_api".into(),
                        category: SmellCategory::Security,
                        severity: Severity::Warning,
                        location: Location {
                            path: ctx.file.path.clone(),
                            start_line: i + 1,
                            start_col: col,
                            end_line: i + 1,
                            end_col: col + pat.len(),
                            name: None,
                        },
                        message: format!("Potentially dangerous: `{pat}` — {msg}"),
                        suggested_refactorings: vec!["Use a safe alternative".into()],
                        ..Default::default()
                    });
                    break; // one finding per line
                }
            }
        }
        findings
    }
}

/// Heuristic: pattern is likely inside a string literal if preceded by a quote.
fn is_in_string(line: &str, pat: &str) -> bool {
    if let Some(pos) = line.find(pat) {
        let before = &line[..pos];
        let quotes = before.matches('"').count();
        quotes % 2 == 1 // odd number of quotes = inside string
    } else {
        false
    }
}

// cha:ignore unsafe_api
fn patterns_for(lang: &str) -> Vec<(&'static str, &'static str)> {
    match lang {
        "rust" => vec![("unsafe ", "unsafe block/fn — review for memory safety")],
        "python" => vec![
            ("eval(", "eval() executes arbitrary code"),
            ("exec(", "exec() executes arbitrary code"),
            ("os.system(", "os.system() is vulnerable to shell injection"),
            ("subprocess.call(", "prefer subprocess.run with shell=False"),
            (
                "pickle.load",
                "pickle deserialization can execute arbitrary code",
            ),
        ],
        "typescript" | "javascript" => vec![
            ("eval(", "eval() executes arbitrary code"),
            ("innerHTML", "innerHTML can lead to XSS"),
            (
                "dangerouslySetInnerHTML",
                "React escape hatch — review for XSS",
            ),
            ("document.write(", "document.write can lead to XSS"),
        ],
        "c" | "cpp" => vec![
            ("gets(", "gets() has no bounds checking — use fgets()"),
            (
                "sprintf(",
                "sprintf() has no bounds checking — use snprintf()",
            ),
            ("strcpy(", "strcpy() has no bounds checking — use strncpy()"),
            ("strcat(", "strcat() has no bounds checking — use strncat()"),
            ("system(", "system() is vulnerable to shell injection"),
        ],
        "go" => vec![
            ("exec.Command(", "review for command injection"),
            ("template.HTML(", "bypasses HTML escaping — review for XSS"),
        ],
        _ => vec![],
    }
}
