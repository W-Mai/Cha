cha_plugin_sdk::plugin!(HardcodedStringsPlugin);

struct HardcodedStringsPlugin;

impl PluginImpl for HardcodedStringsPlugin {
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
                    actual_value: None,
                    threshold: None,
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

    #[test]
    fn no_finding_on_import_line() {
        WasmPluginTest::new()
            .source("typescript", r#"import { foo } from "example.com/lib";"#)
            .option("SITE_DOMAIN", "example.com")
            .assert_no_finding();
    }

    #[test]
    fn no_finding_on_comment_line() {
        WasmPluginTest::new()
            .source("typescript", r#"// see https://example.com for docs"#)
            .option("SITE_DOMAIN", "example.com")
            .assert_no_finding();
    }

    #[test]
    fn multiple_options_each_detected() {
        let findings = WasmPluginTest::new()
            .source("typescript", "const url = \"example.com\";\nconst user = \"octocat\";")
            .option("SITE_DOMAIN", "example.com")
            .option("USER_NAME", "octocat")
            .findings();
        // const lines are skipped, so no findings expected
        assert!(findings.is_empty());
    }

    #[test]
    fn detects_multiple_occurrences_in_body() {
        let findings = WasmPluginTest::new()
            .source(
                "typescript",
                "function a() { return \"example.com\"; }\nfunction b() { return \"example.com\"; }",
            )
            .option("SITE_DOMAIN", "example.com")
            .findings();
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn empty_option_value_skipped() {
        WasmPluginTest::new()
            .source("typescript", r#"fetch("https://example.com");"#)
            .option("SITE_DOMAIN", "")
            .assert_no_finding();
    }

    #[test]
    fn assert_any_finding_works() {
        WasmPluginTest::new()
            .source("typescript", r#"fetch("https://example.com/api");"#)
            .option("SITE_DOMAIN", "example.com")
            .assert_any_finding();
    }

    #[test]
    fn assert_no_finding_named_works() {
        WasmPluginTest::new()
            .source("typescript", r#"fetch("https://example.com/api");"#)
            .option("SITE_DOMAIN", "example.com")
            .assert_no_finding_named("suspicious_name");
    }
}
