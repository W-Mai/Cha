mod common;

use cha_core::plugins::*;
use common::*;

#[test]
fn message_chain_triggers() {
    let mut f = func("deep", 10, 1, false);
    f.chain_depth = 4;
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&MessageChainAnalyzer::default(), &model);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "message_chain");
}

#[test]
fn message_chain_at_threshold() {
    let mut f = func("ok", 10, 1, false);
    f.chain_depth = 3;
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&MessageChainAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn message_chain_shallow() {
    let mut f = func("shallow", 10, 1, false);
    f.chain_depth = 1;
    let model = make_model(vec![f], vec![], vec![], 10);
    let findings = analyze(&MessageChainAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
