mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

fn iface(name: &str) -> ClassInfo {
    let mut c = class(name, 2, 10, true);
    c.is_interface = true;
    c
}

fn implementor(name: &str, parent: &str) -> ClassInfo {
    let mut c = class(name, 2, 10, false);
    c.parent_name = Some(parent.into());
    c
}

#[test]
fn speculative_generality_no_impl() {
    // Interface with 0 implementations in same file
    let model = make_model(vec![], vec![iface("IRepository")], vec![], 10);
    let findings = analyze(&SpeculativeGeneralityAnalyzer::default(), &model);
    assert!(
        findings
            .iter()
            .any(|f| f.smell_name == "speculative_generality")
    );
}

#[test]
fn speculative_generality_one_impl() {
    // Interface with exactly 1 implementation
    let model = make_model(
        vec![],
        vec![iface("IRepository"), implementor("Repo", "IRepository")],
        vec![],
        20,
    );
    let findings = analyze(&SpeculativeGeneralityAnalyzer::default(), &model);
    assert!(
        findings
            .iter()
            .any(|f| f.smell_name == "speculative_generality")
    );
}

#[test]
fn interface_with_multiple_impls_ok() {
    // Interface with 2+ implementations — not flagged
    let model = make_model(
        vec![],
        vec![
            iface("IRepository"),
            implementor("SqlRepo", "IRepository"),
            implementor("MemRepo", "IRepository"),
        ],
        vec![],
        30,
    );
    let findings = analyze(&SpeculativeGeneralityAnalyzer::default(), &model);
    assert!(
        !findings
            .iter()
            .any(|f| f.smell_name == "speculative_generality")
    );
}

#[test]
fn non_interface_not_flagged() {
    let model = make_model(vec![], vec![class("Service", 5, 30, true)], vec![], 30);
    let findings = analyze(&SpeculativeGeneralityAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
