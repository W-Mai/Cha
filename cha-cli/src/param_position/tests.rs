use super::*;
use cha_core::{SourceModel, TypeOrigin, TypeRef};
use std::path::PathBuf;

fn tref_local(name: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin: TypeOrigin::Local,
    }
}

fn tref_primitive(name: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin: TypeOrigin::Primitive,
    }
}

fn func(name: &str, params: Vec<TypeRef>) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        start_line: 1,
        end_line: 1,
        parameter_count: params.len(),
        parameter_types: params,
        ..Default::default()
    }
}

fn model_with(functions: Vec<FunctionInfo>) -> SourceModel {
    SourceModel {
        language: "rust".into(),
        total_lines: 10,
        functions,
        classes: vec![],
        imports: vec![],
        comments: vec![],
        type_aliases: vec![],
    }
}

#[test]
fn flags_same_type_at_different_positions() {
    // `User` at position #1 twice and #2 once — the #2 sig is flagged.
    let m = model_with(vec![
        func("send", vec![tref_local("User"), tref_primitive("String")]),
        func("update", vec![tref_local("User"), tref_primitive("i32")]),
        func("notify", vec![tref_primitive("String"), tref_local("User")]),
    ]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("src/api.rs"), m)]);
    let findings = detect(&idx);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("notify"));
    assert!(findings[0].message.contains("User"));
    assert!(findings[0].message.contains("position #2"));
}

#[test]
fn ignores_consistent_position() {
    let m = model_with(vec![
        func("send", vec![tref_local("User"), tref_primitive("String")]),
        func("update", vec![tref_local("User"), tref_primitive("bool")]),
    ]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("src/api.rs"), m)]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}

#[test]
fn ignores_primitive_types() {
    // String appears at multiple positions across 3 functions — primitives
    // are always skipped, so it doesn't fire even though Id (a local type)
    // is inconsistent and does.
    let m = model_with(vec![
        func("a", vec![tref_primitive("String"), tref_local("Id")]),
        func("b", vec![tref_local("Id"), tref_primitive("String")]),
        func("c", vec![tref_local("Id"), tref_primitive("String")]),
    ]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("src/api.rs"), m)]);
    let findings = detect(&idx);
    // Id at #2 (a), #1 (b), #1 (c): canonical #1, `a` is flagged.
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("Id"));
    assert!(findings[0].message.contains("`a`"));
    assert!(!findings[0].message.contains("String"));
}

#[test]
fn ignores_single_usage() {
    // Only one function takes User — nothing to be inconsistent with.
    let m = model_with(vec![
        func("send", vec![tref_local("User")]),
        func("other", vec![tref_local("Message")]),
    ]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("src/api.rs"), m)]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}

#[test]
fn flags_across_files() {
    let a = model_with(vec![
        func("send", vec![tref_local("User"), tref_primitive("String")]),
        func("update", vec![tref_local("User"), tref_primitive("i32")]),
    ]);
    let b = model_with(vec![func(
        "broadcast",
        vec![tref_primitive("i32"), tref_local("User")],
    )]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("src/a.rs"), a),
        (PathBuf::from("src/b.rs"), b),
    ]);
    let findings = detect(&idx);
    // Three usages of User: two at #1, one at #2 → broadcast is flagged.
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("broadcast"));
    assert!(findings[0].message.contains("position #2"));
}

#[test]
fn skips_mutable_reference_out_params() {
    // A `&mut Vec<Finding>` "sink" at the end of each function is the Rust
    // out-param convention — its position shouldn't be grouped with owned
    // `Finding` parameters elsewhere.
    let mut_sink = TypeRef {
        name: "Finding".into(),
        raw: "&mut Vec<Finding>".into(),
        origin: TypeOrigin::Local,
    };
    let owned = tref_local("Finding");
    let m = model_with(vec![
        func("take_owned_a", vec![owned.clone(), tref_primitive("i32")]),
        func("take_owned_b", vec![owned.clone(), tref_primitive("i32")]),
        func(
            "append_a",
            vec![
                tref_primitive("i32"),
                tref_primitive("i32"),
                mut_sink.clone(),
            ],
        ),
        func(
            "append_b",
            vec![
                tref_primitive("i32"),
                tref_primitive("i32"),
                mut_sink.clone(),
            ],
        ),
    ]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("src/lib.rs"), m)]);
    let findings = detect(&idx);
    // The out-param sinks at position #3 are skipped; the two owned usages
    // at #1 are consistent. No inconsistency signal.
    assert!(
        findings.is_empty(),
        "mutable refs are out-params, not real inconsistency"
    );
}

#[test]
fn skips_self_receiver() {
    // First-position `self` shouldn't be grouped with non-method usages.
    let self_ref = TypeRef {
        name: "Self".into(),
        raw: "&self".into(),
        origin: TypeOrigin::Local,
    };
    let m = model_with(vec![
        func("method_a", vec![self_ref.clone(), tref_local("Foo")]),
        func("method_b", vec![self_ref.clone(), tref_local("Foo")]),
    ]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("src/impl.rs"), m)]);
    let findings = detect(&idx);
    assert!(
        findings.is_empty(),
        "self is not a real parameter for ordering purposes"
    );
}
