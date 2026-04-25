use super::*;
use cha_core::{SourceModel, TypeOrigin, TypeRef};
use std::path::PathBuf;

fn tref(name: &str, origin: TypeOrigin) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin,
    }
}

fn exported_fn(name: &str, params: Vec<TypeRef>, ret: Option<TypeRef>) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        is_exported: true,
        start_line: 1,
        end_line: 1,
        parameter_count: params.len(),
        parameter_types: params,
        return_type: ret,
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
fn flags_third_party_external_in_signature() {
    let m = model_with(vec![exported_fn(
        "parse",
        vec![],
        Some(tref("Value", TypeOrigin::External("serde_json".into()))),
    )]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("cha-parser/src/lib.rs"), m)]);
    let findings = detect(&idx);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("serde_json"));
    assert!(findings[0].message.contains("return type"));
}

#[test]
fn ignores_intra_workspace_crate() {
    // cha-parser's function returns a cha_core::TypeRef. Both crates are in
    // the project, so this isn't a "third-party" leak — it's a
    // workspace-internal dependency that consumers don't additionally
    // depend on.
    let parser_model = model_with(vec![exported_fn(
        "resolve",
        vec![],
        Some(tref("TypeRef", TypeOrigin::External("cha_core".into()))),
    )]);
    let core_model = model_with(vec![]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("cha-parser/src/lib.rs"), parser_model),
        (PathBuf::from("cha-core/src/lib.rs"), core_model),
    ]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}

#[test]
fn ignores_stdlib() {
    let m = model_with(vec![exported_fn(
        "config_path",
        vec![],
        Some(tref("PathBuf", TypeOrigin::External("std".into()))),
    )]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("cha-cli/src/lib.rs"), m)]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}

#[test]
fn ignores_private_fn() {
    let mut f = exported_fn(
        "helper",
        vec![],
        Some(tref("Value", TypeOrigin::External("serde_json".into()))),
    );
    f.is_exported = false;
    let m = model_with(vec![f]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("cha-cli/src/lib.rs"), m)]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}

#[test]
fn flags_parameter_position() {
    let m = model_with(vec![exported_fn(
        "build",
        vec![tref("Node", TypeOrigin::External("tree_sitter".into()))],
        None,
    )]);
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("cha-parser/src/lib.rs"), m)]);
    let findings = detect(&idx);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("parameter #1"));
    assert!(findings[0].message.contains("tree_sitter"));
}

#[test]
fn nested_module_paths_map_to_crate_root() {
    // module `cha_core::model::TypeRef` — root `cha_core`, which matches
    // the workspace crate.
    let m = model_with(vec![exported_fn(
        "use_nested",
        vec![],
        Some(tref(
            "TypeRef",
            TypeOrigin::External("cha_core::model".into()),
        )),
    )]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("cha-parser/src/lib.rs"), m),
        (PathBuf::from("cha-core/src/lib.rs"), model_with(vec![])),
    ]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}
