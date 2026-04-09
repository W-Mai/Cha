mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

fn data_class(name: &str, fields: usize, methods: usize) -> ClassInfo {
    let mut c = class(name, methods, 20, false);
    c.field_count = fields;
    c.has_behavior = false;
    c
}

fn behavior_class(name: &str) -> ClassInfo {
    let mut c = class(name, 5, 30, false);
    c.field_count = 3;
    c.has_behavior = true;
    c
}

#[test]
fn data_class_detected() {
    let model = make_model(vec![], vec![data_class("UserDto", 5, 0)], vec![], 20);
    let findings = analyze(&DataClassAnalyzer::default(), &model);
    assert!(findings.iter().any(|f| f.smell_name == "data_class"));
}

#[test]
fn behavior_class_ok() {
    let model = make_model(vec![], vec![behavior_class("UserService")], vec![], 30);
    let findings = analyze(&DataClassAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn empty_class_not_flagged_as_data_class() {
    let model = make_model(vec![], vec![class("Empty", 0, 5, false)], vec![], 5);
    let findings = analyze(&DataClassAnalyzer::default(), &model);
    assert!(!findings.iter().any(|f| f.smell_name == "data_class"));
}
