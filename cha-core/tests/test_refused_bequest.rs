mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

fn subclass(name: &str, methods: usize, overrides: usize) -> ClassInfo {
    let mut c = class(name, methods, 30, false);
    c.override_count = overrides;
    c.parent_name = Some("Base".into());
    c
}

#[test]
fn refused_bequest_detected() {
    // 8 methods, 7 overrides = 87.5% override rate
    let model = make_model(vec![], vec![subclass("Child", 8, 7)], vec![], 30);
    let findings = analyze(&RefusedBequestAnalyzer::default(), &model);
    assert!(findings.iter().any(|f| f.smell_name == "refused_bequest"));
}

#[test]
fn low_override_rate_ok() {
    let model = make_model(vec![], vec![subclass("Child", 10, 2)], vec![], 30);
    let findings = analyze(&RefusedBequestAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn no_parent_not_flagged() {
    let model = make_model(vec![], vec![class("Standalone", 8, 30, false)], vec![], 30);
    let findings = analyze(&RefusedBequestAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
