use super::*;
use cha_core::{FunctionInfo, SourceModel};
use std::path::PathBuf;

fn func(name: &str, arms: Vec<ArmValue>) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        start_line: 1,
        end_line: 10,
        switch_arm_values: arms,
        ..Default::default()
    }
}

fn model_with(f: FunctionInfo) -> SourceModel {
    SourceModel {
        language: "rust".into(),
        total_lines: 10,
        functions: vec![f],
        classes: vec![],
        imports: vec![],
        comments: vec![],
        type_aliases: vec![],
    }
}

fn run_on(m: SourceModel) -> Vec<Finding> {
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("t.rs"), m)]);
    detect(&idx)
}

#[test]
fn three_string_arms_flags() {
    let m = model_with(func(
        "dispatch",
        vec![
            ArmValue::Str("add".into()),
            ArmValue::Str("sub".into()),
            ArmValue::Str("mul".into()),
        ],
    ));
    let findings = run_on(m);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "stringly_typed_dispatch");
    assert!(findings[0].message.contains("string"));
    assert!(findings[0].message.contains("add"));
}

#[test]
fn three_int_arms_flags() {
    let m = model_with(func(
        "parse_status",
        vec![ArmValue::Int(200), ArmValue::Int(404), ArmValue::Int(500)],
    ));
    let findings = run_on(m);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("integer"));
    assert!(findings[0].message.contains("200"));
}

#[test]
fn two_arms_below_threshold_quiet() {
    let m = model_with(func(
        "binary_check",
        vec![ArmValue::Str("yes".into()), ArmValue::Str("no".into())],
    ));
    assert!(run_on(m).is_empty());
}

#[test]
fn char_arms_quiet() {
    // C tokenizer pattern — char case labels are not a stringly-typed
    // smell, they're character classification.
    let m = model_with(func(
        "is_vowel",
        vec![
            ArmValue::Char('a'),
            ArmValue::Char('e'),
            ArmValue::Char('i'),
            ArmValue::Char('o'),
            ArmValue::Char('u'),
        ],
    ));
    assert!(run_on(m).is_empty());
}

#[test]
fn enum_variant_arms_quiet() {
    // Rust `match` on an enum generates `Other` arm values since the
    // patterns aren't literal nodes.
    let m = model_with(func(
        "match_event",
        vec![ArmValue::Other, ArmValue::Other, ArmValue::Other],
    ));
    assert!(run_on(m).is_empty());
}

#[test]
fn mixed_literal_and_default_still_flags_strings() {
    // 3 string arms + default (`_` → `Other`). Still fires.
    let m = model_with(func(
        "router",
        vec![
            ArmValue::Str("get".into()),
            ArmValue::Str("post".into()),
            ArmValue::Str("put".into()),
            ArmValue::Other,
        ],
    ));
    assert_eq!(run_on(m).len(), 1);
}

#[test]
fn multiple_switches_aggregate() {
    // Two switches in one function, 2 string arms each — only 4 total
    // but MIN_LITERAL_ARMS is 3, so this fires (aggregation by design).
    let m = model_with(func(
        "big_dispatcher",
        vec![
            ArmValue::Str("a".into()),
            ArmValue::Str("b".into()),
            ArmValue::Str("c".into()),
            ArmValue::Str("d".into()),
        ],
    ));
    assert_eq!(run_on(m).len(), 1);
}
