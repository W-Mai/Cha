mod common;

use cha_core::plugins::*;
use common::*;

#[test]
fn primitive_obsession_triggers() {
    let mut f = func("prim", 10, 1, false);
    f.parameter_types = vec!["i32".into(), "String".into(), "bool".into()];
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&PrimitiveObsessionAnalyzer::default(), &model);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "primitive_obsession");
}

#[test]
fn primitive_obsession_mixed_types() {
    let mut f = func("mixed", 10, 1, false);
    f.parameter_types = vec!["i32".into(), "MyStruct".into(), "bool".into()];
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&PrimitiveObsessionAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn primitive_obsession_too_few_params() {
    let mut f = func("few", 10, 1, false);
    f.parameter_types = vec!["i32".into(), "bool".into()];
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&PrimitiveObsessionAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
