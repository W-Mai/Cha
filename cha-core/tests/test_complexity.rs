mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

#[test]
fn complexity_warning() {
    let model = make_model(vec![func("complex", 10, 10, false)], vec![], vec![], 10);
    let findings = analyze(&ComplexityAnalyzer::default(), &model);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Warning);
}

#[test]
fn complexity_error() {
    let model = make_model(
        vec![func("very_complex", 10, 20, false)],
        vec![],
        vec![],
        10,
    );
    let findings = analyze(&ComplexityAnalyzer::default(), &model);
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn complexity_below_threshold() {
    let model = make_model(vec![func("simple", 10, 9, false)], vec![], vec![], 10);
    let findings = analyze(&ComplexityAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
