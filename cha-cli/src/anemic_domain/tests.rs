use super::*;
use cha_core::{SourceModel, TypeOrigin, TypeRef};

fn tref(name: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin: TypeOrigin::Local,
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

fn anemic_class(name: &str, field_count: usize) -> ClassInfo {
    ClassInfo {
        name: name.into(),
        start_line: 1,
        end_line: 5,
        field_count,
        has_behavior: false,
        is_interface: false,
        ..Default::default()
    }
}

fn model(functions: Vec<FunctionInfo>, classes: Vec<ClassInfo>) -> SourceModel {
    SourceModel {
        language: "rust".into(),
        total_lines: 10,
        functions,
        classes,
        imports: vec![],
        comments: vec![],
        type_aliases: vec![],
    }
}

/// Build a two-file project (class file + consumer file) and run detection.
fn run_two_files(
    class_path: &str,
    class: ClassInfo,
    consumer_path: &str,
    consumer: FunctionInfo,
) -> Vec<Finding> {
    let models = vec![
        (PathBuf::from(class_path), model(vec![], vec![class])),
        (PathBuf::from(consumer_path), model(vec![consumer], vec![])),
    ];
    detect_from_models(&models)
}

#[test]
fn flags_anemic_class_with_external_service_by_filename() {
    let findings = run_two_files(
        "src/order.rs",
        anemic_class("Order", 4),
        "src/order_service.rs",
        func("do_thing", vec![tref("Order")]),
    );
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "anemic_domain_model");
    assert!(findings[0].message.contains("Order"));
    assert!(findings[0].message.contains("do_thing"));
}

#[test]
fn flags_anemic_class_with_verb_prefixed_function() {
    // Function name has service verb prefix → strong signal even without
    // a service-ish filename.
    let findings = run_two_files(
        "src/invoice.rs",
        anemic_class("Invoice", 3),
        "src/totals.rs",
        func("calculate_total", vec![tref("Invoice")]),
    );
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("calculate_total"));
}

#[test]
fn ignores_non_anemic_classes() {
    // A class that owns its behavior, or is just an interface, shouldn't
    // be flagged even when paired with a service-style function.
    let cases: [(&str, fn(&mut ClassInfo)); 2] = [
        ("Order", |c| c.has_behavior = true),
        ("Handler", |c| c.is_interface = true),
    ];
    for (name, mutate) in cases {
        let mut klass = anemic_class(name, 3);
        mutate(&mut klass);
        let findings = run_two_files(
            "src/class.rs",
            klass,
            "src/class_service.rs",
            func(&format!("process_{name}"), vec![tref(name)]),
        );
        assert!(findings.is_empty(), "case `{name}` should be ignored");
    }
}

#[test]
fn ignores_below_field_threshold() {
    let findings = run_two_files(
        "src/tiny.rs",
        anemic_class("Tiny", 1),
        "src/svc.rs",
        func("process_tiny", vec![tref("Tiny")]),
    );
    assert!(findings.is_empty());
}

#[test]
fn ignores_when_no_external_service_uses_class() {
    // Anemic class exists, but nothing out there operates on it — just a
    // data_class case, not a full anemic-domain anti-pattern.
    let klass = anemic_class("Config", 5);
    let class_model = model(vec![], vec![klass]);
    let models = vec![(PathBuf::from("src/config.rs"), class_model)];
    let findings = detect_from_models(&models);
    assert!(findings.is_empty());
}

#[test]
fn ignores_function_in_same_file_as_class() {
    // A verb-prefixed function living in the same file as the class is
    // treated as part of the class's own module (impl block etc.), not an
    // external service — avoid flagging Rust `struct + impl` as anemic.
    let klass = anemic_class("User", 4);
    let own_fn = func("process_user", vec![tref("User")]);
    let combined = model(vec![own_fn], vec![klass]);
    let models = vec![(PathBuf::from("src/user.rs"), combined)];
    let findings = detect_from_models(&models);
    assert!(findings.is_empty());
}

#[test]
fn ignores_unrelated_functions() {
    // Neither the function name nor the filename signals a service — just
    // a bystander that happens to accept the type.
    let findings = run_two_files(
        "src/order.rs",
        anemic_class("Order", 4),
        "src/logger.rs",
        func("log", vec![tref("Order")]),
    );
    assert!(findings.is_empty());
}
