mod common;

use cha_core::plugins::*;
use common::*;

#[test]
fn switch_statement_triggers() {
    let mut f = func("big_match", 20, 1, false);
    f.switch_arms = 9;
    let model = make_model(vec![f], vec![], vec![], 20);
    let findings = analyze(&SwitchStatementAnalyzer::default(), &model);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "switch_statement");
}

#[test]
fn switch_statement_at_threshold() {
    let mut f = func("ok_match", 20, 1, false);
    f.switch_arms = 8;
    let model = make_model(vec![f], vec![], vec![], 20);
    let findings = analyze(&SwitchStatementAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn switch_statement_no_arms() {
    let model = make_model(vec![func("plain", 10, 1, false)], vec![], vec![], 10);
    let findings = analyze(&SwitchStatementAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
