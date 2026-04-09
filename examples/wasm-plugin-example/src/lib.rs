cha_plugin_sdk::plugin!(ExamplePlugin);

struct ExamplePlugin;

impl Guest for ExamplePlugin {
    fn name() -> String {
        "example-wasm".into()
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
                    end_line: f.end_line,
                    name: Some(f.name.clone()),
                },
                message: format!(
                    "Function `{}` has a suspicious name suggesting incomplete work",
                    f.name
                ),
                suggested_refactorings: vec!["Rename Method".into()],
            })
            .collect()
    }
}
