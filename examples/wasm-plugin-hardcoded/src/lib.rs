cha_plugin_sdk::plugin!(HardcodedStringsPlugin);

struct HardcodedStringsPlugin;

impl Guest for HardcodedStringsPlugin {
    fn name() -> String {
        "hardcoded-strings".into()
    }

    fn analyze(input: AnalysisInput) -> Vec<Finding> {
        let pairs: Vec<(&str, &str)> = cha_plugin_sdk::str_options!(&input.options).collect();
        if pairs.is_empty() {
            return vec![];
        }

        let mut findings = Vec::new();
        for (i, line) in input.content.lines().enumerate() {
            if is_skip_line(line) {
                continue;
            }
            for (const_name, literal) in &pairs {
                if literal.is_empty() || !line.contains(literal) {
                    continue;
                }
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
                    suggested_refactorings: vec![format!(
                        "Replace with reference to `{}`",
                        const_name
                    )],
                });
            }
        }
        findings
    }
}

fn is_skip_line(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("import ")
        || t.starts_with("export const ")
        || t.starts_with("const ")
        || t.starts_with("//")
        || t.starts_with('*')
        || t.starts_with("/*")
}

#[cfg(test)]
mod tests {
    use cha_plugin_sdk::test_utils::WasmPluginTest;

    #[test]
    fn detects_hardcoded_string() {
        WasmPluginTest::new()
            .source("typescript", r#"fetch("https://example.com/api");"#)
            .option("SITE_DOMAIN", "example.com")
            .assert_finding("hardcoded_string");
    }

    #[test]
    fn no_finding_without_options() {
        WasmPluginTest::new()
            .source("typescript", r#"fetch("https://example.com/api");"#)
            .assert_no_finding();
    }

    #[test]
    fn no_finding_on_const_declaration() {
        WasmPluginTest::new()
            .source("typescript", r#"const SITE_DOMAIN = "example.com";"#)
            .option("SITE_DOMAIN", "example.com")
            .assert_no_finding();
    }
}
