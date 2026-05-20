cha_plugin_sdk::plugin!(ExamplePlugin);

struct ExamplePlugin;

impl PluginImpl for ExamplePlugin {
    fn name() -> String {
        "example-wasm".into()
    }

    fn smells() -> Vec<String> {
        vec![
            "suspicious_name".into(),
            "unsafe_block".into(),
            "unused_helper".into(),
        ]
    }

    fn analyze(input: AnalysisInput) -> Vec<Finding> {
        let mut findings = Vec::new();

        for f in &input.functions {
            let lower = f.name.to_lowercase();
            if lower.contains("todo") || lower.contains("fixme") || lower.contains("hack") {
                findings.push(Finding {
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
                });
            }
        }

        if input.role == FileRole::Test {
            return findings;
        }

        // unsafe_block — uses tree-query host import
        let matches = tree_query::run_query("(unsafe_block) @unsafe");
        for m in &matches {
            for capture in m {
                findings.push(Finding {
                    smell_name: "unsafe_block".into(),
                    category: SmellCategory::Security,
                    severity: Severity::Hint,
                    location: Location {
                        path: input.path.clone(),
                        start_line: capture.start_line,
                        start_col: capture.start_col,
                        end_line: capture.end_line,
                        end_col: capture.end_col,
                        name: None,
                    },
                    message: "unsafe block detected — review for soundness".into(),
                    suggested_refactorings: vec!["Encapsulate unsafe in a safe wrapper".into()],
                    actual_value: None,
                    threshold: None,
                });
            }
        }

        // unused_helper — uses project-query host import.
        // A function whose name starts with `_` (private convention) AND
        // has no callers anywhere in the project (including its own file).
        for f in &input.functions {
            if !f.name.starts_with('_') || f.is_exported {
                continue;
            }
            // callers_of gives the full set across the project, including
            // same-file callers. Empty means truly unused.
            if project_query::callers_of(&f.name).is_empty() {
                findings.push(Finding {
                    smell_name: "unused_helper".into(),
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
                        "Helper `{}` (underscore-prefixed) has no callers in the project",
                        f.name
                    ),
                    suggested_refactorings: vec!["Remove dead code".into()],
                    actual_value: None,
                    threshold: None,
                });
            }
        }

        findings
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

    #[test]
    fn no_unsafe_in_test_file() {
        WasmPluginTest::new()
            .source("rust", "fn foo() { unsafe { std::ptr::null::<u8>().read() }; }")
            .assert_no_finding_named("unsafe_block");
    }
}
