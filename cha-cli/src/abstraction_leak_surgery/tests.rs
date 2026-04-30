//! Unit tests for the non-git parts — pair-keying, external-type set
//! construction, shared-type intersection. Git-driven co-change logic
//! is exercised by the real cha-self baseline; mocking `git log` in a
//! unit test would be more plumbing than signal.

use super::*;
use cha_core::{FunctionInfo, SourceModel, TypeRef};
use std::path::PathBuf;

fn tref_ext(name: &str, module: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin: TypeOrigin::External(module.into()),
    }
}

fn tref_local(name: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin: TypeOrigin::Local,
    }
}

fn func(name: &str, params: Vec<TypeRef>, ret: Option<TypeRef>) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        parameter_types: params,
        return_type: ret,
        ..Default::default()
    }
}

fn model(functions: Vec<FunctionInfo>) -> SourceModel {
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
fn external_type_set_collects_params_and_returns() {
    let m = model(vec![
        func("a", vec![tref_ext("Value", "serde_json")], None),
        func("b", vec![], Some(tref_ext("Node", "tree_sitter"))),
        func("c", vec![tref_local("LocalThing")], None),
    ]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("some-crate/f.rs"), m)]);
    let ws = workspace_crate_names(&idx);
    let out = build_external_type_index(&idx, &ws);
    let set = out.get(&PathBuf::from("some-crate/f.rs")).unwrap();
    assert!(set.contains("serde_json::Value"));
    assert!(set.contains("tree_sitter::Node"));
    assert!(
        !set.iter().any(|s| s.contains("LocalThing")),
        "local types must not leak into the external set"
    );
    assert_eq!(set.len(), 2);
}

#[test]
fn files_without_external_types_are_omitted() {
    let m = model(vec![func("a", vec![tref_local("T")], None)]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("f.rs"), m)]);
    let ws = workspace_crate_names(&idx);
    let out = build_external_type_index(&idx, &ws);
    assert!(
        out.is_empty(),
        "purely-local file should not enter the index"
    );
}

#[test]
fn workspace_sibling_crate_skipped() {
    // Two files at different top-level directories act like workspace
    // crates; a type whose External module root matches one of those
    // directories should be filtered out.
    let parser_model = model(vec![func("f", vec![tref_ext("Finding", "cha_core")], None)]);
    let core_model = model(vec![]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("cha-parser/src/lib.rs"), parser_model),
        (PathBuf::from("cha-core/src/lib.rs"), core_model),
    ]);
    let ws = workspace_crate_names(&idx);
    let out = build_external_type_index(&idx, &ws);
    assert!(
        out.is_empty(),
        "workspace-sibling type references should not count as external"
    );
}

#[test]
fn pair_key_is_order_independent() {
    let a = Path::new("a.rs");
    let b = Path::new("b.rs");
    assert_eq!(pair_key(a, b), pair_key(b, a));
}

#[test]
fn commit_group_parser_splits_on_blank_lines() {
    let text = "a.rs\nb.rs\n\nc.rs\n\na.rs\nc.rs\n";
    let groups = split_commit_groups(text);
    assert_eq!(groups.len(), 3);
    assert_eq!(
        groups[0],
        vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")]
    );
    assert_eq!(groups[1], vec![PathBuf::from("c.rs")]);
    assert_eq!(
        groups[2],
        vec![PathBuf::from("a.rs"), PathBuf::from("c.rs")]
    );
}
