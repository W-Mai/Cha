use super::*;
use cha_core::{FunctionInfo, SourceModel, TypeOrigin, TypeRef};
use std::path::PathBuf;

fn tref(name: &str, origin: TypeOrigin) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin,
    }
}

fn func(
    name: &str,
    params: Vec<(&str, TypeRef)>,
    chain_depth: usize,
    external_refs: Vec<&str>,
) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        start_line: 1,
        end_line: 1,
        chain_depth,
        parameter_count: params.len(),
        parameter_types: params.iter().map(|(_, t)| t.clone()).collect(),
        parameter_names: params.iter().map(|(n, _)| (*n).to_string()).collect(),
        external_refs: external_refs.into_iter().map(String::from).collect(),
        ..Default::default()
    }
}

fn run(f: FunctionInfo) -> Vec<Finding> {
    let m = SourceModel {
        language: "rust".into(),
        total_lines: 10,
        functions: vec![f],
        classes: vec![],
        imports: vec![],
        comments: vec![],
        type_aliases: vec![],
    };
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("t.rs"), m)]);
    detect(&idx)
}

#[test]
fn chain_on_external_param_flags() {
    let findings = run(func(
        "walk",
        vec![(
            "node",
            tref("Node", TypeOrigin::External("tree_sitter".into())),
        )],
        3,
        vec!["node", "child_by_field_name"],
    ));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "cross_boundary_chain");
    assert!(findings[0].message.contains("tree_sitter"));
    assert!(findings[0].message.contains("node"));
    assert_eq!(findings[0].actual_value, Some(3.0));
}

#[test]
fn chain_on_local_param_quiet() {
    let findings = run(func(
        "process",
        vec![("config", tref("Config", TypeOrigin::Local))],
        4,
        vec!["config", "inner", "build"],
    ));
    assert!(
        findings.is_empty(),
        "local-typed chains are `message_chain` territory, not cross-boundary"
    );
}

#[test]
fn shallow_chain_quiet() {
    let findings = run(func(
        "probe",
        vec![(
            "ctx",
            tref("Node", TypeOrigin::External("tree_sitter".into())),
        )],
        2,
        vec!["ctx", "kind"],
    ));
    assert!(findings.is_empty(), "depth 2 is below MIN_DEPTH");
}

#[test]
fn external_param_unused_quiet() {
    // External param exists but body chain happens elsewhere. We
    // don't have per-chain root info; requiring the param name in
    // external_refs guards against this false positive.
    let findings = run(func(
        "collect",
        vec![(
            "node",
            tref("Node", TypeOrigin::External("tree_sitter".into())),
        )],
        4,
        vec!["self", "inner", "vec"],
    ));
    assert!(findings.is_empty());
}

#[test]
fn multi_params_external_wins_over_local() {
    // Function has a local param AND an external param, both used.
    // The external one should drive the finding.
    let findings = run(func(
        "merge",
        vec![
            ("config", tref("Config", TypeOrigin::Local)),
            (
                "node",
                tref("Node", TypeOrigin::External("tree_sitter".into())),
            ),
        ],
        3,
        vec!["config", "node", "child_count"],
    ));
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("node"));
    assert!(findings[0].message.contains("tree_sitter"));
}

#[test]
fn non_external_origins_quiet() {
    // Primitives, Local, and Unknown origins all fail the "External"
    // check — none of them should fire even with a deep chain, because
    // none represent a crossed third-party boundary.
    for (origin_label, origin) in [
        ("Primitive", TypeOrigin::Primitive),
        ("Unknown", TypeOrigin::Unknown),
    ] {
        let findings = run(func(
            "probe",
            vec![("x", tref("T", origin.clone()))],
            4,
            vec!["x"],
        ));
        assert!(
            findings.is_empty(),
            "{origin_label}-origin chains should stay quiet, got {findings:?}"
        );
    }
}

#[test]
fn workspace_internal_crate_quiet() {
    // When the "external" module is actually another workspace crate,
    // it's not crossing a third-party boundary — it's an internal
    // dependency of this project. `ProjectIndex::from_models` derives
    // workspace crates from the top-level path component, so adding
    // a cha_core-rooted file next to the cha-parser one marks
    // cha_core as workspace-internal.
    let parser_model = SourceModel {
        language: "rust".into(),
        total_lines: 10,
        functions: vec![func(
            "walk",
            vec![(
                "finding",
                tref("Finding", TypeOrigin::External("cha_core".into())),
            )],
            3,
            vec!["finding", "location"],
        )],
        classes: vec![],
        imports: vec![],
        comments: vec![],
        type_aliases: vec![],
    };
    let core_model = SourceModel {
        language: "rust".into(),
        total_lines: 1,
        functions: vec![],
        classes: vec![],
        imports: vec![],
        comments: vec![],
        type_aliases: vec![],
    };
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("cha-parser/src/lib.rs"), parser_model),
        (PathBuf::from("cha-core/src/lib.rs"), core_model),
    ]);
    assert!(
        detect(&idx).is_empty(),
        "sibling workspace crate should not count as cross-boundary"
    );
}

