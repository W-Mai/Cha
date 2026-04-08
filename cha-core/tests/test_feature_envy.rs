mod common;

use cha_core::plugins::*;
use common::*;

#[test]
fn feature_envy_triggers() {
    let mut f = func("envious", 10, 1, false);
    f.external_refs = vec!["db".into(), "db".into(), "db".into()];
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&FeatureEnvyAnalyzer::default(), &model);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "feature_envy");
}

#[test]
fn feature_envy_spread_refs() {
    let mut f = func("spread", 10, 1, false);
    f.external_refs = vec!["a".into(), "b".into(), "c".into()];
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&FeatureEnvyAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn feature_envy_too_few_refs() {
    let mut f = func("few", 10, 1, false);
    f.external_refs = vec!["db".into(), "db".into()];
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&FeatureEnvyAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
