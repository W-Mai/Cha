mod common;

use cha_core::plugins::*;
use common::*;

#[test]
fn naming_too_short() {
    let model = make_model(vec![func("x", 5, 1, false)], vec![], vec![], 5);
    let findings = analyze(&NamingAnalyzer::default(), &model);
    assert!(findings.iter().any(|f| f.smell_name == "naming_too_short"));
}

#[test]
fn naming_class_lowercase() {
    let model = make_model(vec![], vec![class("myClass", 0, 5, false)], vec![], 5);
    let findings = analyze(&NamingAnalyzer::default(), &model);
    assert!(findings.iter().any(|f| f.smell_name == "naming_convention"));
}

#[test]
fn naming_ok() {
    let model = make_model(
        vec![func("process_data", 5, 1, false)],
        vec![class("DataProcessor", 0, 5, false)],
        vec![],
        10,
    );
    let findings = analyze(&NamingAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
