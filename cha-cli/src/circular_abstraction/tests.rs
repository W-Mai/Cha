use super::*;
use cha_core::{FunctionInfo, SourceModel};

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
fn flags_bidirectional_calls() {
    // a.rs calls b.rs's b1 and b2; b.rs calls a.rs's a1 and a2.
    let a = model_with(vec![
        func("a1", vec![]),
        func("a2", vec![]),
        func("caller_in_a", vec!["b1", "b2"]),
    ]);
    let b = model_with(vec![
        func("b1", vec![]),
        func("b2", vec![]),
        func("caller_in_b", vec!["a1", "a2"]),
    ]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("src/a.rs"), a),
        (PathBuf::from("src/b.rs"), b),
    ]);
    let findings = detect(&idx);
    assert_eq!(findings.len(), 2);
    assert!(
        findings
            .iter()
            .all(|f| f.smell_name == "circular_abstraction")
    );
}

#[test]
fn ignores_one_way_call() {
    // a calls b twice but b never calls a.
    let a = model_with(vec![
        func("a1", vec![]),
        func("caller_in_a", vec!["b1", "b2"]),
    ]);
    let b = model_with(vec![func("b1", vec![]), func("b2", vec![])]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("src/a.rs"), a),
        (PathBuf::from("src/b.rs"), b),
    ]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}

#[test]
fn ignores_below_threshold() {
    // Each side only calls the other once — below MIN_CALLS_EACH_SIDE.
    let a = model_with(vec![func("a1", vec![]), func("caller_in_a", vec!["b1"])]);
    let b = model_with(vec![func("b1", vec![]), func("caller_in_b", vec!["a1"])]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("src/a.rs"), a),
        (PathBuf::from("src/b.rs"), b),
    ]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}

#[test]
fn ignores_self_calls() {
    // A file calling its own functions is normal; no cycle.
    let m = model_with(vec![
        func("a1", vec![]),
        func("a2", vec!["a1"]),
        func("a3", vec!["a1", "a2"]),
    ]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("src/a.rs"), m)]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}

#[test]
fn reports_each_pair_once_even_with_many_call_sites() {
    // a has many callers into b, b has many callers into a. Still one
    // pair (two findings, one per side).
    let a = model_with(vec![
        func("a1", vec![]),
        func("a2", vec![]),
        func("call1", vec!["b1", "b2"]),
        func("call2", vec!["b1", "b2"]),
        func("call3", vec!["b2"]),
    ]);
    let b = model_with(vec![
        func("b1", vec![]),
        func("b2", vec![]),
        func("callback1", vec!["a1", "a2"]),
        func("callback2", vec!["a1", "a2"]),
    ]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("src/a.rs"), a),
        (PathBuf::from("src/b.rs"), b),
    ]);
    let findings = detect(&idx);
    assert_eq!(findings.len(), 2);
}

#[test]
fn unresolved_callees_do_not_fabricate_cycles() {
    // If neither side's calls resolve to a project file, no cycle.
    let a = model_with(vec![
        func("a1", vec![]),
        func("caller", vec!["println", "format", "to_string"]),
    ]);
    let b = model_with(vec![func("b1", vec![]), func("caller_b", vec!["println"])]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("src/a.rs"), a),
        (PathBuf::from("src/b.rs"), b),
    ]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}
