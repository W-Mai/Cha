use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

/// Detect usage of potentially dangerous functions and constructs.
///
/// Uses tree-sitter AST queries (when `ctx.tree` is available) so matches
/// inside string literals, comments, and identifiers are not falsely flagged.
/// Falls back to nothing when there's no AST — better silence than noise.
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

    fn smells(&self) -> Vec<String> {
        vec!["unsafe_api".into()]
    }

    fn description(&self) -> &str {
        "Dangerous function calls (eval/exec/system)"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let (Some(tree), Some(lang)) = (ctx.tree, ctx.ts_language) else {
            return vec![];
        };
        let patterns = queries_for(&ctx.model.language);
        if patterns.is_empty() {
            return vec![];
        }
        let source = ctx.file.content.as_bytes();
        let mut findings = Vec::new();
        for (pattern, label, msg) in patterns {
            for matches in crate::query::run_query(tree, lang, source, pattern) {
                let Some(cap) = matches
                    .iter()
                    .find(|c| c.capture_name == "site")
                    .or_else(|| matches.first())
                else {
                    continue;
                };
                findings.push(Finding {
                    smell_name: "unsafe_api".into(),
                    category: SmellCategory::Security,
                    severity: Severity::Warning,
                    location: Location {
                        path: ctx.file.path.clone(),
                        start_line: cap.start_line as usize,
                        start_col: cap.start_col as usize,
                        end_line: cap.end_line as usize,
                        end_col: cap.end_col as usize,
                        name: None,
                    },
                    message: format!("Potentially dangerous: `{label}` — {msg}"),
                    suggested_refactorings: vec!["Use a safe alternative".into()],
                    ..Default::default()
                });
            }
        }
        findings
    }
}

/// Tree-sitter S-expr queries per language. Each entry is
/// `(query_pattern, display_label, message)`.
///
/// Queries should produce a `@site` capture for the location to report.
// cha:ignore long_method
fn queries_for(lang: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    match lang {
        "rust" => vec![
            (
                "(unsafe_block) @site",
                "unsafe block",
                "unsafe block — review for memory safety",
            ),
            (
                "(function_modifiers \"unsafe\") @site",
                "unsafe fn",
                "unsafe fn — review for memory safety",
            ),
        ],
        "python" => vec![
            (
                r#"(call function: (identifier) @n (#eq? @n "eval")) @site"#,
                "eval",
                "eval() executes arbitrary code",
            ),
            (
                r#"(call function: (identifier) @n (#eq? @n "exec")) @site"#,
                "exec",
                "exec() executes arbitrary code",
            ),
            (
                r#"(call function: (attribute object: (identifier) @o attribute: (identifier) @a) (#eq? @o "os") (#eq? @a "system")) @site"#,
                "os.system",
                "os.system() is vulnerable to shell injection",
            ),
            (
                r#"(call function: (attribute object: (identifier) @o attribute: (identifier) @a) (#eq? @o "subprocess") (#eq? @a "call")) @site"#,
                "subprocess.call",
                "prefer subprocess.run with shell=False",
            ),
            (
                r#"(call function: (attribute object: (identifier) @o attribute: (identifier) @a) (#eq? @o "pickle") (#match? @a "^(load|loads)$")) @site"#,
                "pickle.load",
                "pickle deserialization can execute arbitrary code",
            ),
        ],
        "typescript" => vec![
            (
                r#"(call_expression function: (identifier) @n (#eq? @n "eval")) @site"#,
                "eval",
                "eval() executes arbitrary code",
            ),
            (
                r#"(member_expression property: (property_identifier) @p (#eq? @p "innerHTML")) @site"#,
                "innerHTML",
                "innerHTML can lead to XSS",
            ),
            (
                r#"(jsx_attribute (property_identifier) @p (#eq? @p "dangerouslySetInnerHTML")) @site"#,
                "dangerouslySetInnerHTML",
                "React escape hatch — review for XSS",
            ),
            (
                r#"(call_expression function: (member_expression object: (identifier) @o property: (property_identifier) @p) (#eq? @o "document") (#eq? @p "write")) @site"#,
                "document.write",
                "document.write can lead to XSS",
            ),
        ],
        "c" | "cpp" => vec![
            (
                r#"(call_expression function: (identifier) @n (#eq? @n "gets")) @site"#,
                "gets",
                "gets() has no bounds checking — use fgets()",
            ),
            (
                r#"(call_expression function: (identifier) @n (#eq? @n "sprintf")) @site"#,
                "sprintf",
                "sprintf() has no bounds checking — use snprintf()",
            ),
            (
                r#"(call_expression function: (identifier) @n (#eq? @n "strcpy")) @site"#,
                "strcpy",
                "strcpy() has no bounds checking — use strncpy()",
            ),
            (
                r#"(call_expression function: (identifier) @n (#eq? @n "strcat")) @site"#,
                "strcat",
                "strcat() has no bounds checking — use strncat()",
            ),
            (
                r#"(call_expression function: (identifier) @n (#eq? @n "system")) @site"#,
                "system",
                "system() is vulnerable to shell injection",
            ),
        ],
        "go" => vec![
            (
                r#"(call_expression function: (selector_expression operand: (identifier) @o field: (field_identifier) @f) (#eq? @o "exec") (#eq? @f "Command")) @site"#,
                "exec.Command",
                "review for command injection",
            ),
            (
                r#"(call_expression function: (selector_expression operand: (identifier) @o field: (field_identifier) @f) (#eq? @o "template") (#eq? @f "HTML")) @site"#,
                "template.HTML",
                "bypasses HTML escaping — review for XSS",
            ),
        ],
        _ => vec![],
    }
}
