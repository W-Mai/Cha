mod common;

use cha_core::plugins::*;
use common::*;

#[test]
fn duplicate_triggers() {
    let model = make_model(
        vec![func_with_hash("a", 15, 42), func_with_hash("b", 15, 42)],
        vec![],
        vec![],
        10,
    );
    let findings = analyze(&DuplicateCodeAnalyzer, &model);
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].smell_name, "duplicate_code");
}

#[test]
fn duplicate_different_hash_no_trigger() {
    let model = make_model(
        vec![func_with_hash("a", 15, 42), func_with_hash("b", 15, 99)],
        vec![],
        vec![],
        10,
    );
    let findings = analyze(&DuplicateCodeAnalyzer, &model);
    assert!(findings.is_empty());
}

#[test]
fn duplicate_short_fn_ignored() {
    let model = make_model(
        vec![func_with_hash("a", 3, 42), func_with_hash("b", 3, 42)],
        vec![],
        vec![],
        6,
    );
    let findings = analyze(&DuplicateCodeAnalyzer, &model);
    assert!(findings.is_empty());
}
