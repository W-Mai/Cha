mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

#[test]
fn length_long_method_triggers() {
    let model = make_model(vec![func("big", 51, 1, false)], vec![], vec![], 51);
    let findings = analyze(&LengthAnalyzer::default(), &model);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "long_method");
    assert_eq!(findings[0].severity, Severity::Hint); // risk=1.02, Hint
}

#[test]
fn length_at_threshold_no_trigger() {
    // 49 lines = risk 0.98 < 1.0, should not trigger
    let model = make_model(vec![func("ok", 49, 1, false)], vec![], vec![], 49);
    let findings = analyze(&LengthAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn length_double_threshold_error() {
    // 200 lines, complexity 1: risk = 4.0 → Error
    let model = make_model(vec![func("huge", 200, 1, false)], vec![], vec![], 200);
    let findings = analyze(&LengthAnalyzer::default(), &model);
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn length_large_class() {
    let model = make_model(vec![], vec![class("Big", 11, 201, false)], vec![], 201);
    let findings = analyze(&LengthAnalyzer::default(), &model);
    assert!(findings.iter().any(|f| f.smell_name == "large_class"));
}
