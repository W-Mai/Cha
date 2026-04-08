mod common;

use std::path::PathBuf;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

#[test]
fn layer_violation_triggers() {
    let analyzer = LayerViolationAnalyzer::from_config_str("domain:0,service:1,controller:2");
    let file = SourceFile::new(PathBuf::from("domain/repo.rs"), String::new());
    let model = make_model(vec![], vec![], vec![import("controller/handler", 1)], 5);
    let ctx = AnalysisContext {
        file: &file,
        model: &model,
    };
    let findings = analyzer.analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn layer_violation_same_layer_ok() {
    let analyzer = LayerViolationAnalyzer::from_config_str("domain:0,service:1");
    let file = SourceFile::new(PathBuf::from("service/a.rs"), String::new());
    let model = make_model(vec![], vec![], vec![import("service/b", 1)], 5);
    let ctx = AnalysisContext {
        file: &file,
        model: &model,
    };
    let findings = analyzer.analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn layer_violation_no_config_no_trigger() {
    let model = make_model(vec![], vec![], vec![import("anything", 1)], 5);
    let findings = analyze(&LayerViolationAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
