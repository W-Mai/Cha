cha_plugin_sdk::plugin!(ExamplePlugin);

struct ExamplePlugin;

impl PluginImpl for ExamplePlugin {
    fn name() -> String {
        "example-wasm".into()
    }

    fn smells() -> Vec<String> {
        vec!["suspicious_name".into()]
    }

    fn analyze(input: AnalysisInput) -> Vec<Finding> {
        input
            .functions
            .iter()
            .filter(|f| {
                let lower = f.name.to_lowercase();
                lower.contains("todo") || lower.contains("fixme") || lower.contains("hack")
            })
            .map(|f| Finding {
                smell_name: "suspicious_name".into(),
                category: SmellCategory::Dispensables,
                severity: Severity::Hint,
                location: Location {
                    path: input.path.clone(),
                    start_line: f.start_line,
                    start_col: f.name_col,
                    end_line: f.start_line,
                    end_col: f.name_end_col,
                    name: Some(f.name.clone()),
                },
                message: format!(
                    "Function `{}` has a suspicious name suggesting incomplete work",
                    f.name
                ),
                suggested_refactorings: vec!["Rename Method".into()],
                actual_value: None,
                threshold: None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use cha_plugin_sdk::test_utils::WasmPluginTest;

    #[test]
    fn detects_suspicious_name() {
        WasmPluginTest::new()
            .source("typescript", "function todo_fix() {}")
            .assert_finding("suspicious_name");
    }

    #[test]
    fn no_finding_for_clean_name() {
        WasmPluginTest::new()
            .source("typescript", "function processData() {}")
            .assert_no_finding();
    }

    #[test]
    fn detects_fixme_in_name() {
        WasmPluginTest::new()
            .source("typescript", "function fixme_later() {}")
            .assert_finding("suspicious_name");
    }

    #[test]
    fn detects_hack_in_name() {
        WasmPluginTest::new()
            .source("typescript", "function hack_workaround() {}")
            .assert_finding("suspicious_name");
    }

    #[test]
    fn multiple_functions_only_suspicious_flagged() {
        let findings = WasmPluginTest::new()
            .source("typescript", "function processData() {}\nfunction todo_cleanup() {}")
            .findings();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "suspicious_name");
    }

    #[test]
    fn empty_source_no_finding() {
        WasmPluginTest::new()
            .source("typescript", "")
            .assert_no_finding();
    }

    #[test]
    fn assert_any_finding_passes_when_finding_exists() {
        WasmPluginTest::new()
            .source("typescript", "function todo_fix() {}")
            .assert_any_finding();
    }

    #[test]
    fn assert_no_finding_named_passes_for_different_smell() {
        WasmPluginTest::new()
            .source("typescript", "function todo_fix() {}")
            .assert_no_finding_named("high_complexity");
    }
}
