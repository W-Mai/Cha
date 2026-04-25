use super::*;
use cha_core::SourceModel;

fn func(name: &str, calls: Vec<&str>) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        start_line: 1,
        end_line: 1,
        called_functions: calls.into_iter().map(String::from).collect(),
        ..Default::default()
    }
}

fn model_with(functions: Vec<FunctionInfo>) -> SourceModel {
    SourceModel {
        language: "rust".into(),
        total_lines: 10,
        functions,
        classes: vec![],
        imports: vec![],
        comments: vec![],
        type_aliases: vec![],
    }
}

#[test]
fn flags_function_that_calls_external_module_heavily() {
    let orchestrator = model_with(vec![func("run", vec!["tick", "render", "commit"])]);
    // These three live in another file.
    let view = model_with(vec![
        func("tick", vec![]),
        func("render", vec![]),
        func("commit", vec![]),
    ]);
    let models = vec![
        (PathBuf::from("src/controller.rs"), orchestrator),
        (PathBuf::from("src/view.rs"), view),
    ];
    let findings = detect_from_models(&models);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "module_envy");
    assert!(findings[0].message.contains("run"));
    assert!(findings[0].message.contains("view.rs"));
}

#[test]
fn ignores_when_calls_are_balanced() {
    // run calls 3 into view.rs but also 3 into its own file — balanced
    // coordinator, not envious.
    let controller = model_with(vec![
        func(
            "run",
            vec!["tick", "render", "commit", "validate", "authorise", "log"],
        ),
        func("validate", vec![]),
        func("authorise", vec![]),
        func("log", vec![]),
    ]);
    let view = model_with(vec![
        func("tick", vec![]),
        func("render", vec![]),
        func("commit", vec![]),
    ]);
    let models = vec![
        (PathBuf::from("src/controller.rs"), controller),
        (PathBuf::from("src/view.rs"), view),
    ];
    let findings = detect_from_models(&models);
    assert!(findings.is_empty());
}

#[test]
fn ignores_below_threshold() {
    // Only 2 external calls — below MIN_EXTERNAL_CALLS.
    let orchestrator = model_with(vec![func("run", vec!["tick", "render"])]);
    let view = model_with(vec![func("tick", vec![]), func("render", vec![])]);
    let models = vec![
        (PathBuf::from("src/controller.rs"), orchestrator),
        (PathBuf::from("src/view.rs"), view),
    ];
    let findings = detect_from_models(&models);
    assert!(findings.is_empty());
}

#[test]
fn unresolvable_callees_are_ignored() {
    // Calls to functions that aren't in any project file (stdlib, third
    // party) don't count toward envy.
    let f = model_with(vec![func(
        "run",
        vec!["println", "format", "clone", "to_string"],
    )]);
    let models = vec![(PathBuf::from("src/main.rs"), f)];
    let findings = detect_from_models(&models);
    assert!(findings.is_empty());
}

#[test]
fn ignores_known_helper_patterns() {
    // Two well-known false-positive shapes are suppressed:
    //   1. test file → common.rs (same tests directory)
    //   2. any file   → utils.rs (by convention, a shared helper)
    let cases: [(&str, &str); 2] = [
        ("tests/test_scenario.rs", "tests/common.rs"),
        ("src/service.rs", "src/utils.rs"),
    ];
    for (caller, callee_file) in cases {
        let caller_model = model_with(vec![func("op", vec!["helper_a", "helper_b", "helper_c"])]);
        let callee_model = model_with(vec![
            func("helper_a", vec![]),
            func("helper_b", vec![]),
            func("helper_c", vec![]),
        ]);
        let models = vec![
            (PathBuf::from(caller), caller_model),
            (PathBuf::from(callee_file), callee_model),
        ];
        let findings = detect_from_models(&models);
        assert!(
            findings.is_empty(),
            "case `{caller}` → `{callee_file}` should be suppressed"
        );
    }
}

#[test]
fn flags_the_most_envied_module_when_multiple() {
    let orchestrator = model_with(vec![func(
        "run",
        vec!["tick", "render", "commit", "fetch", "parse"],
    )]);
    let view = model_with(vec![
        func("tick", vec![]),
        func("render", vec![]),
        func("commit", vec![]),
    ]);
    let io = model_with(vec![func("fetch", vec![]), func("parse", vec![])]);
    let models = vec![
        (PathBuf::from("src/controller.rs"), orchestrator),
        (PathBuf::from("src/view.rs"), view),
        (PathBuf::from("src/io.rs"), io),
    ];
    let findings = detect_from_models(&models);
    assert_eq!(findings.len(), 1);
    // view.rs has 3 calls, io.rs has 2 — view wins.
    assert!(findings[0].message.contains("view.rs"));
}
