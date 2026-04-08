mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

#[test]
fn middle_man_triggers() {
    let c = ClassInfo {
        name: "Proxy".into(),
        start_line: 1,
        end_line: 10,
        method_count: 4,
        line_count: 10,
        is_exported: false,
        delegating_method_count: 3,
    };
    let model = make_model(vec![], vec![c], vec![], 10);
    let findings = analyze(&MiddleManAnalyzer::default(), &model);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "middle_man");
}

#[test]
fn middle_man_below_ratio() {
    let c = ClassInfo {
        name: "Mixed".into(),
        start_line: 1,
        end_line: 10,
        method_count: 4,
        line_count: 10,
        is_exported: false,
        delegating_method_count: 1,
    };
    let model = make_model(vec![], vec![c], vec![], 10);
    let findings = analyze(&MiddleManAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn middle_man_too_few_methods() {
    let c = ClassInfo {
        name: "Tiny".into(),
        start_line: 1,
        end_line: 5,
        method_count: 2,
        line_count: 5,
        is_exported: false,
        delegating_method_count: 2,
    };
    let model = make_model(vec![], vec![c], vec![], 5);
    let findings = analyze(&MiddleManAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
