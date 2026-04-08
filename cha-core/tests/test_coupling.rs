mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

#[test]
fn coupling_warning() {
    let imports: Vec<_> = (0..16)
        .map(|i| import(&format!("mod_{i}"), i + 1))
        .collect();
    let model = make_model(vec![], vec![], imports, 20);
    let findings = analyze(&CouplingAnalyzer::default(), &model);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Warning);
}

#[test]
fn coupling_error_double() {
    let imports: Vec<_> = (0..31)
        .map(|i| import(&format!("mod_{i}"), i + 1))
        .collect();
    let model = make_model(vec![], vec![], imports, 40);
    let findings = analyze(&CouplingAnalyzer::default(), &model);
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn coupling_at_threshold_no_trigger() {
    let imports: Vec<_> = (0..15)
        .map(|i| import(&format!("mod_{i}"), i + 1))
        .collect();
    let model = make_model(vec![], vec![], imports, 20);
    let findings = analyze(&CouplingAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
