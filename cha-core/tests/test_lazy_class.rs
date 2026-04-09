mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

fn lazy(name: &str, methods: usize, lines: usize) -> ClassInfo {
    class(name, methods, lines, false)
}

#[test]
fn lazy_class_detected() {
    // 0 methods, 8 lines — qualifies as lazy
    let model = make_model(vec![], vec![lazy("Wrapper", 0, 8)], vec![], 8);
    let findings = analyze(&LazyClassAnalyzer::default(), &model);
    assert!(findings.iter().any(|f| f.smell_name == "lazy_class"));
}

#[test]
fn class_with_methods_ok() {
    let model = make_model(vec![], vec![lazy("Service", 5, 50)], vec![], 50);
    let findings = analyze(&LazyClassAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn single_method_small_class_detected() {
    let model = make_model(vec![], vec![lazy("Helper", 1, 6)], vec![], 6);
    let findings = analyze(&LazyClassAnalyzer::default(), &model);
    assert!(findings.iter().any(|f| f.smell_name == "lazy_class"));
}
