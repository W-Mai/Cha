mod common;

use cha_core::plugins::*;
use common::*;

#[test]
fn long_param_list_triggers() {
    let mut f = func("many_params", 10, 1, false);
    f.parameter_count = 6;
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&LongParameterListAnalyzer::default(), &model);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "long_parameter_list");
}

#[test]
fn long_param_list_at_threshold() {
    let mut f = func("ok", 10, 1, false);
    f.parameter_count = 5;
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&LongParameterListAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn long_param_list_below() {
    let mut f = func("few", 10, 1, false);
    f.parameter_count = 2;
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&LongParameterListAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
