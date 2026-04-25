use super::*;
use cha_core::SourceModel;

fn func(name: &str, params: Vec<TypeRef>, calls: Vec<String>) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        start_line: 1,
        end_line: 1,
        parameter_count: params.len(),
        parameter_types: params,
        called_functions: calls,
        ..Default::default()
    }
}

fn tref(name: &str, origin: TypeOrigin) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin,
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
fn flags_external_type_in_callback_group() {
    let ts = tref("Node", TypeOrigin::External("tree_sitter".into()));
    let callbacks = vec![
        func("on_a", vec![ts.clone()], vec![]),
        func("on_b", vec![ts.clone()], vec![]),
        func("on_c", vec![ts.clone()], vec![]),
    ];
    let dispatcher = func(
        "dispatch",
        vec![],
        vec!["on_a".into(), "on_b".into(), "on_c".into()],
    );
    let mut all = callbacks;
    all.push(dispatcher);
    let models = vec![(PathBuf::from("test.rs"), model(all))];
    let findings = detect_from_models(&models, 3);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "abstraction_boundary_leak");
    assert!(
        findings[0].message.contains("tree_sitter"),
        "message should cite the external module: {}",
        findings[0].message
    );
}

#[test]
fn ignores_group_of_local_types() {
    let local = tref("Finding", TypeOrigin::Local);
    let callbacks = vec![
        func("check_a", vec![local.clone()], vec![]),
        func("check_b", vec![local.clone()], vec![]),
        func("check_c", vec![local.clone()], vec![]),
    ];
    let dispatcher = func(
        "analyze",
        vec![],
        vec!["check_a".into(), "check_b".into(), "check_c".into()],
    );
    let mut all = callbacks;
    all.push(dispatcher);
    let models = vec![(PathBuf::from("test.rs"), model(all))];
    let findings = detect_from_models(&models, 3);
    assert!(findings.is_empty(), "local types should not trigger");
}

#[test]
fn ignores_divergent_signatures() {
    // 3 siblings but each with different signature — no group reaches min_size.
    let dispatcher = func("dispatch", vec![], vec!["a".into(), "b".into(), "c".into()]);
    let ts = |n| tref("Node", TypeOrigin::External("tree_sitter".into()));
    let callbacks = vec![
        func("a", vec![ts("a")], vec![]),
        func(
            "b",
            vec![
                tref("Node", TypeOrigin::External("tree_sitter".into())),
                tref("Node", TypeOrigin::External("tree_sitter".into())),
            ],
            vec![],
        ),
        func("c", vec![], vec![]),
    ];
    let mut all = callbacks;
    all.push(dispatcher);
    let models = vec![(PathBuf::from("test.rs"), model(all))];
    let findings = detect_from_models(&models, 3);
    assert!(findings.is_empty());
}

#[test]
fn ignores_below_threshold() {
    let ts = tref("Node", TypeOrigin::External("tree_sitter".into()));
    let callbacks = vec![
        func("on_a", vec![ts.clone()], vec![]),
        func("on_b", vec![ts.clone()], vec![]),
    ];
    let dispatcher = func("dispatch", vec![], vec!["on_a".into(), "on_b".into()]);
    let mut all = callbacks;
    all.push(dispatcher);
    let models = vec![(PathBuf::from("test.rs"), model(all))];
    let findings = detect_from_models(&models, 3);
    assert!(findings.is_empty(), "2 callbacks < min_group_size 3");
}

#[test]
fn unknown_origin_reported_with_qualifier() {
    let u = tref("cmark_node_t", TypeOrigin::Unknown);
    let callbacks = vec![
        func("on_a", vec![u.clone()], vec![]),
        func("on_b", vec![u.clone()], vec![]),
        func("on_c", vec![u.clone()], vec![]),
    ];
    let dispatcher = func(
        "dispatch",
        vec![],
        vec!["on_a".into(), "on_b".into(), "on_c".into()],
    );
    let mut all = callbacks;
    all.push(dispatcher);
    let models = vec![(PathBuf::from("test.c"), model(all))];
    let findings = detect_from_models(&models, 3);
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0].message.contains("unresolved"),
        "unknown origin should include lower-confidence qualifier: {}",
        findings[0].message
    );
}

#[test]
fn rename_suggestion_skipped_when_prefix_uniform() {
    let ts = tref("Node", TypeOrigin::External("tree_sitter".into()));
    let callbacks = vec![
        func("on_a", vec![ts.clone()], vec![]),
        func("on_b", vec![ts.clone()], vec![]),
        func("on_c", vec![ts.clone()], vec![]),
    ];
    let dispatcher = func(
        "dispatch",
        vec![],
        vec!["on_a".into(), "on_b".into(), "on_c".into()],
    );
    let mut all = callbacks;
    all.push(dispatcher);
    let models = vec![(PathBuf::from("test.rs"), model(all))];
    let findings = detect_from_models(&models, 3);
    let last = findings[0].suggested_refactorings.last().unwrap();
    assert!(
        last.contains("already uses"),
        "expected shared-convention hint, got: {last}"
    );
}

// --- return_type_leak ---

fn func_with_return(
    name: &str,
    params: Vec<TypeRef>,
    calls: Vec<String>,
    ret: Option<TypeRef>,
) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        start_line: 1,
        end_line: 1,
        parameter_count: params.len(),
        parameter_types: params,
        called_functions: calls,
        return_type: ret,
        ..Default::default()
    }
}

#[test]
fn flags_external_return_type_in_callback_group() {
    let param = tref("Ctx", TypeOrigin::Local);
    let external_ret = tref("Value", TypeOrigin::External("serde_json".into()));
    let callbacks = vec![
        func_with_return(
            "on_a",
            vec![param.clone()],
            vec![],
            Some(external_ret.clone()),
        ),
        func_with_return(
            "on_b",
            vec![param.clone()],
            vec![],
            Some(external_ret.clone()),
        ),
        func_with_return(
            "on_c",
            vec![param.clone()],
            vec![],
            Some(external_ret.clone()),
        ),
    ];
    let dispatcher = func_with_return(
        "dispatch",
        vec![],
        vec!["on_a".into(), "on_b".into(), "on_c".into()],
        None,
    );
    let mut all = callbacks;
    all.push(dispatcher);
    let models = vec![(PathBuf::from("test.rs"), model(all))];
    let findings = detect_from_models(&models, 3);
    let rtl: Vec<_> = findings
        .iter()
        .filter(|f| f.smell_name == "return_type_leak")
        .collect();
    assert_eq!(rtl.len(), 1, "expected one return_type_leak finding");
    assert!(
        rtl[0].message.contains("serde_json"),
        "msg: {}",
        rtl[0].message
    );
}

#[test]
fn ignores_local_return_type() {
    let param = tref("Ctx", TypeOrigin::Local);
    let local_ret = tref("Finding", TypeOrigin::Local);
    let callbacks = vec![
        func_with_return("on_a", vec![param.clone()], vec![], Some(local_ret.clone())),
        func_with_return("on_b", vec![param.clone()], vec![], Some(local_ret.clone())),
        func_with_return("on_c", vec![param.clone()], vec![], Some(local_ret.clone())),
    ];
    let dispatcher = func_with_return(
        "dispatch",
        vec![],
        vec!["on_a".into(), "on_b".into(), "on_c".into()],
        None,
    );
    let mut all = callbacks;
    all.push(dispatcher);
    let models = vec![(PathBuf::from("test.rs"), model(all))];
    let findings = detect_from_models(&models, 3);
    assert!(
        findings.iter().all(|f| f.smell_name != "return_type_leak"),
        "local return types should not trigger"
    );
}

#[test]
fn ignores_divergent_return_types() {
    let param = tref("Ctx", TypeOrigin::Local);
    let ra = tref("A", TypeOrigin::External("ext".into()));
    let rb = tref("B", TypeOrigin::External("ext".into()));
    let callbacks = vec![
        func_with_return("on_a", vec![param.clone()], vec![], Some(ra.clone())),
        func_with_return("on_b", vec![param.clone()], vec![], Some(rb.clone())),
        func_with_return("on_c", vec![param.clone()], vec![], Some(ra.clone())),
    ];
    let dispatcher = func_with_return(
        "dispatch",
        vec![],
        vec!["on_a".into(), "on_b".into(), "on_c".into()],
        None,
    );
    let mut all = callbacks;
    all.push(dispatcher);
    let models = vec![(PathBuf::from("test.rs"), model(all))];
    let findings = detect_from_models(&models, 3);
    assert!(
        findings.iter().all(|f| f.smell_name != "return_type_leak"),
        "handlers return different external types, not a unified leak"
    );
}

#[test]
fn return_type_leak_independent_from_param_leak() {
    let local_param = tref("Ctx", TypeOrigin::Local);
    let external_ret = tref("Value", TypeOrigin::External("serde_json".into()));
    let callbacks = vec![
        func_with_return(
            "h_a",
            vec![local_param.clone()],
            vec![],
            Some(external_ret.clone()),
        ),
        func_with_return(
            "h_b",
            vec![local_param.clone()],
            vec![],
            Some(external_ret.clone()),
        ),
        func_with_return(
            "h_c",
            vec![local_param.clone()],
            vec![],
            Some(external_ret.clone()),
        ),
    ];
    let dispatcher = func_with_return(
        "run",
        vec![],
        vec!["h_a".into(), "h_b".into(), "h_c".into()],
        None,
    );
    let mut all = callbacks;
    all.push(dispatcher);
    let models = vec![(PathBuf::from("test.rs"), model(all))];
    let findings = detect_from_models(&models, 3);
    // Param leak should NOT fire (local Ctx); only RTL fires.
    let abl_count = findings
        .iter()
        .filter(|f| f.smell_name == "abstraction_boundary_leak")
        .count();
    let rtl_count = findings
        .iter()
        .filter(|f| f.smell_name == "return_type_leak")
        .count();
    assert_eq!(abl_count, 0);
    assert_eq!(rtl_count, 1);
}

// --- test_only_type_in_production ---

fn model_with_classes(
    functions: Vec<FunctionInfo>,
    classes: Vec<cha_core::ClassInfo>,
) -> cha_core::SourceModel {
    cha_core::SourceModel {
        language: "rust".into(),
        total_lines: 10,
        functions,
        classes,
        imports: vec![],
        comments: vec![],
        type_aliases: vec![],
    }
}

fn class(name: &str) -> cha_core::ClassInfo {
    cha_core::ClassInfo {
        name: name.into(),
        start_line: 1,
        end_line: 5,
        ..Default::default()
    }
}

#[test]
fn test_only_path_detection() {
    assert!(is_test_path(&PathBuf::from("src/tests/foo.rs")));
    assert!(is_test_path(&PathBuf::from("crate/test/module.rs")));
    assert!(is_test_path(&PathBuf::from("src/__tests__/x.ts")));
    assert!(is_test_path(&PathBuf::from("foo/bar/test_utils.py")));
    assert!(is_test_path(&PathBuf::from("module/handler_test.go")));
    assert!(is_test_path(&PathBuf::from("components/Button.test.ts")));
    assert!(!is_test_path(&PathBuf::from("src/lib.rs")));
    assert!(!is_test_path(&PathBuf::from("src/analyze.rs")));
}

#[test]
fn flags_test_only_type_in_production_signature() {
    // Test file defines MockClient; production file uses it as a param.
    let test_model = model_with_classes(vec![], vec![class("MockClient")]);
    let prod_fn = func_with_return(
        "send_request",
        vec![tref("MockClient", TypeOrigin::Local)],
        vec![],
        None,
    );
    let prod_model = model_with_classes(vec![prod_fn], vec![]);
    let models = vec![
        (PathBuf::from("tests/mocks.rs"), test_model),
        (PathBuf::from("src/service.rs"), prod_model),
    ];
    let findings = detect_from_models(&models, 3);
    let leaks: Vec<_> = findings
        .iter()
        .filter(|f| f.smell_name == "test_only_type_in_production")
        .collect();
    assert_eq!(leaks.len(), 1, "expected 1 test-only leak finding");
    assert!(leaks[0].message.contains("MockClient"));
    assert!(leaks[0].message.contains("parameter #1"));
}

#[test]
fn flags_test_only_type_in_return_position() {
    let test_model = model_with_classes(vec![], vec![class("FakeStore")]);
    let prod_fn = func_with_return(
        "make_store",
        vec![],
        vec![],
        Some(tref("FakeStore", TypeOrigin::Local)),
    );
    let prod_model = model_with_classes(vec![prod_fn], vec![]);
    let models = vec![
        (PathBuf::from("tests/fakes.rs"), test_model),
        (PathBuf::from("src/factory.rs"), prod_model),
    ];
    let findings = detect_from_models(&models, 3);
    let leaks: Vec<_> = findings
        .iter()
        .filter(|f| f.smell_name == "test_only_type_in_production")
        .collect();
    assert_eq!(leaks.len(), 1);
    assert!(leaks[0].message.contains("return type"));
}

#[test]
fn ignores_type_declared_in_both_test_and_prod() {
    // Type is in test file AND production file → not test-only, no flag.
    let test_model = model_with_classes(vec![], vec![class("Shared")]);
    let prod_type_model = model_with_classes(vec![], vec![class("Shared")]);
    let consumer = func_with_return(
        "use_shared",
        vec![tref("Shared", TypeOrigin::Local)],
        vec![],
        None,
    );
    let consumer_model = model_with_classes(vec![consumer], vec![]);
    let models = vec![
        (PathBuf::from("tests/shared.rs"), test_model),
        (PathBuf::from("src/shared.rs"), prod_type_model),
        (PathBuf::from("src/consumer.rs"), consumer_model),
    ];
    let findings = detect_from_models(&models, 3);
    assert!(
        findings
            .iter()
            .all(|f| f.smell_name != "test_only_type_in_production"),
        "shared type should not trigger"
    );
}

#[test]
fn test_file_using_test_only_type_is_fine() {
    // Test file uses its own test-only type — that's what tests are for.
    let test_type = model_with_classes(vec![], vec![class("MockClient")]);
    let test_consumer_fn = func_with_return(
        "with_mock",
        vec![tref("MockClient", TypeOrigin::Local)],
        vec![],
        None,
    );
    let test_consumer = model_with_classes(vec![test_consumer_fn], vec![]);
    let models = vec![
        (PathBuf::from("tests/mocks.rs"), test_type),
        (PathBuf::from("tests/integration.rs"), test_consumer),
    ];
    let findings = detect_from_models(&models, 3);
    assert!(
        findings
            .iter()
            .all(|f| f.smell_name != "test_only_type_in_production"),
        "test-file → test-file use should not trigger"
    );
}
