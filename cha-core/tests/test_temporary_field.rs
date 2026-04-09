mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

fn field_class(name: &str, fields: Vec<&str>, methods: usize) -> ClassInfo {
    let mut c = class(name, methods, 30, false);
    c.field_count = fields.len();
    c.field_names = fields.into_iter().map(String::from).collect();
    c
}

fn func_with_refs(name: &str, refs: Vec<&str>) -> FunctionInfo {
    let mut f = func(name, 10, 1, false);
    f.referenced_fields = refs.into_iter().map(String::from).collect();
    f
}

#[test]
fn temporary_field_detected() {
    // field used in only 1 out of 5 methods
    let model = make_model(
        vec![
            func_with_refs("init", vec!["temp_value"]),
            func("process", 10, 1, false),
            func("validate", 10, 1, false),
            func("save", 10, 1, false),
            func("reset", 10, 1, false),
        ],
        vec![field_class("Service", vec!["temp_value", "name", "id"], 5)],
        vec![],
        50,
    );
    let findings = analyze(&TemporaryFieldAnalyzer::default(), &model);
    assert!(findings.iter().any(|f| f.smell_name == "temporary_field"));
}

#[test]
fn widely_used_field_ok() {
    let model = make_model(
        vec![
            func_with_refs("a", vec!["name"]),
            func_with_refs("b", vec!["name"]),
            func_with_refs("c", vec!["name"]),
        ],
        vec![field_class("Service", vec!["name"], 3)],
        vec![],
        30,
    );
    let findings = analyze(&TemporaryFieldAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
