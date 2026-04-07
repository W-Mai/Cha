wit_bindgen::generate!({
    world: "analyzer",
});

struct ExamplePlugin;

impl Guest for ExamplePlugin {
    fn name() -> String {
        "example-wasm".into()
    }

    fn analyze(input: AnalysisInput) -> Vec<Finding> {
        use cha::plugin::types::{Location, Severity, SmellCategory};

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
                        end_line: f.end_line,
                        name: Some(f.name.clone()),
                    },
                    message: format!(
                        "Function `{}` has a suspicious name suggesting incomplete work",
                        f.name
                    ),
                    suggested_refactorings: vec!["Rename Method".into()],
                });
            }
        }

        findings
    }
}

export!(ExamplePlugin);
