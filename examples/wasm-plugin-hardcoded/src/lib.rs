wit_bindgen::generate!({
    world: "analyzer",
});

struct HardcodedStringsPlugin;

impl Guest for HardcodedStringsPlugin {
    fn name() -> String {
        "hardcoded-strings".into()
    }

    /// Detect hardcoded values that should use named constants.
    ///
    /// Options (via `analysis-input.options`):
    ///   key   = label shown in the finding message
    ///   value = literal string to search for
    ///
    /// Example config (`.cha.toml`):
    /// ```toml
    /// [[wasm_plugins]]
    /// path = "hardcoded-strings.wasm"
    /// [wasm_plugins.options]
    /// SITE_DOMAIN = "example.com"
    /// USER_NAME   = "octocat"
    /// ```
    fn analyze(input: AnalysisInput) -> Vec<Finding> {
        use cha::plugin::types::{Location, Severity, SmellCategory};

        if input.options.is_empty() {
            return vec![];
        }

        let mut findings = Vec::new();

        for (i, line) in input.content.lines().enumerate() {
            if is_skip_line(line) {
                continue;
            }
            for (const_name, literal) in &input.options {
                if literal.is_empty() {
                    continue;
                }
                if line.contains(literal.as_str()) {
                    let line_num = (i + 1) as u32;
                    findings.push(Finding {
                        smell_name: "hardcoded_string".into(),
                        category: SmellCategory::ChangePreventers,
                        severity: Severity::Warning,
                        location: Location {
                            path: input.path.clone(),
                            start_line: line_num,
                            end_line: line_num,
                            name: None,
                        },
                        message: format!(
                            "Hardcoded \"{}\" found — use constant `{}` instead",
                            literal, const_name,
                        ),
                        suggested_refactorings: vec![
                            format!("Replace with reference to `{}`", const_name),
                        ],
                    });
                }
            }
        }

        findings
    }
}

/// Skip import/const declarations and comment lines.
fn is_skip_line(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("import ")
        || t.starts_with("export const ")
        || t.starts_with("const ")
        || t.starts_with("//")
        || t.starts_with('*')
        || t.starts_with("/*")
}

export!(HardcodedStringsPlugin);
