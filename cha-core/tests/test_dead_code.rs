mod common;

use std::path::PathBuf;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

#[test]
fn dead_code_unexported_unreferenced() {
    let file = SourceFile::new(
        PathBuf::from("test.rs"),
        "fn unused() {\n    todo!()\n}\n".into(),
    );
    let model = make_model(vec![func("unused", 3, 1, false)], vec![], vec![], 3);
    let ctx = AnalysisContext {
        file: &file,
        model: &model,
    };
    let findings = DeadCodeAnalyzer.analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "dead_code");
}

#[test]
fn dead_code_exported_no_trigger() {
    let model = make_model(vec![func("public_fn", 3, 1, true)], vec![], vec![], 3);
    let findings = analyze(&DeadCodeAnalyzer, &model);
    assert!(findings.is_empty());
}

#[test]
fn dead_code_main_no_trigger() {
    let model = make_model(vec![func("main", 3, 1, false)], vec![], vec![], 3);
    let findings = analyze(&DeadCodeAnalyzer, &model);
    assert!(findings.is_empty());
}
