use super::*;
use cha_core::{ClassInfo, FunctionInfo, SourceModel, TypeOrigin, TypeRef};
use std::path::PathBuf;

fn raw_tref(raw: &str) -> TypeRef {
    TypeRef {
        name: raw.trim_matches('*').trim().into(),
        raw: raw.into(),
        origin: TypeOrigin::Unknown,
    }
}

fn func(name: &str, first_param_raw: Option<&str>) -> FunctionInfo {
    let params = match first_param_raw {
        Some(r) => vec![raw_tref(r)],
        None => vec![],
    };
    FunctionInfo {
        name: name.into(),
        start_line: 1,
        end_line: 1,
        parameter_count: params.len(),
        parameter_types: params,
        is_exported: true,
        ..Default::default()
    }
}

fn class(name: &str, start_line: usize) -> ClassInfo {
    ClassInfo {
        name: name.into(),
        start_line,
        end_line: start_line + 5,
        ..Default::default()
    }
}

fn c_model(
    functions: Vec<FunctionInfo>,
    classes: Vec<ClassInfo>,
    aliases: Vec<(&str, &str)>,
) -> SourceModel {
    SourceModel {
        language: "c".into(),
        total_lines: 10,
        functions,
        classes,
        imports: vec![],
        comments: vec![],
        type_aliases: aliases
            .into_iter()
            .map(|(a, b)| (a.into(), b.into()))
            .collect(),
    }
}

// ── tokenize ──

#[test]
fn tokenize_snake_case() {
    assert_eq!(
        tokenize("foo_bar_set_width"),
        vec!["foo", "bar", "set", "width"]
    );
    assert_eq!(tokenize("widget_t"), vec!["widget", "t"]);
}

#[test]
fn tokenize_pascal_case() {
    assert_eq!(tokenize("FooBarSetWidth"), vec!["foo", "bar", "set", "width"]);
    assert_eq!(tokenize("Widget"), vec!["widget"]);
}

#[test]
fn tokenize_mixed() {
    assert_eq!(tokenize("Widget_SetX"), vec!["widget", "set", "x"]);
}

#[test]
fn tokenize_acronyms() {
    assert_eq!(tokenize("HTTPRequest"), vec!["http", "request"]);
    assert_eq!(tokenize("parseXMLTree"), vec!["parse", "xml", "tree"]);
    assert_eq!(tokenize("HTTP"), vec!["http"]);
}

#[test]
fn tokenize_leading_underscore() {
    assert_eq!(tokenize("_widget_t"), vec!["widget", "t"]);
}

// ── attribution ──

#[test]
fn attributes_snake_method_via_typedef() {
    let cls = class("_widget_t", 1);
    let header = c_model(vec![], vec![cls], vec![("widget_t", "_widget_t")]);
    let impl_model = c_model(
        vec![
            func("widget_set_x", Some("widget_t *")),
            func("widget_init", Some("widget_t *")),
        ],
        vec![],
        vec![],
    );
    let mut models = vec![
        (PathBuf::from("widget.h"), header),
        (PathBuf::from("widget.c"), impl_model),
    ];
    enrich_c_oop(&mut models);
    let cls = &models[0].1.classes[0];
    assert_eq!(cls.method_count, 2);
    assert!(cls.has_behavior);
}

#[test]
fn rejects_pointer_param_without_matching_prefix() {
    // debug_print takes widget_t* but is clearly not a widget method.
    let header = c_model(
        vec![],
        vec![class("_widget_t", 1)],
        vec![("widget_t", "_widget_t")],
    );
    let caller = c_model(
        vec![func("debug_print", Some("widget_t *"))],
        vec![],
        vec![],
    );
    let mut models = vec![
        (PathBuf::from("widget.h"), header),
        (PathBuf::from("util.c"), caller),
    ];
    enrich_c_oop(&mut models);
    let cls = &models[0].1.classes[0];
    assert_eq!(cls.method_count, 0, "debug_print must not be attributed");
    assert!(!cls.has_behavior);
}

#[test]
fn attributes_pascal_method() {
    let header = c_model(vec![], vec![class("Widget", 1)], vec![]);
    let impl_model = c_model(vec![func("Widget_Init", Some("Widget *"))], vec![], vec![]);
    let mut models = vec![
        (PathBuf::from("widget.h"), header),
        (PathBuf::from("widget.c"), impl_model),
    ];
    enrich_c_oop(&mut models);
    assert_eq!(models[0].1.classes[0].method_count, 1);
}

#[test]
fn attributes_mixed_naming_in_same_project() {
    let header = c_model(vec![], vec![class("Widget", 1)], vec![]);
    let impl_model = c_model(
        vec![
            func("widget_set_x", Some("Widget *")),
            func("Widget_Init", Some("Widget *")),
        ],
        vec![],
        vec![],
    );
    let mut models = vec![
        (PathBuf::from("widget.h"), header),
        (PathBuf::from("widget.c"), impl_model),
    ];
    enrich_c_oop(&mut models);
    assert_eq!(models[0].1.classes[0].method_count, 2);
}

#[test]
fn attributes_short_name_struct() {
    let header = c_model(vec![], vec![class("foo", 1)], vec![]);
    let impl_model = c_model(vec![func("foo_bar", Some("foo *"))], vec![], vec![]);
    let mut models = vec![
        (PathBuf::from("foo.h"), header),
        (PathBuf::from("foo.c"), impl_model),
    ];
    enrich_c_oop(&mut models);
    assert_eq!(models[0].1.classes[0].method_count, 1);
}

#[test]
fn attributes_with_struct_prefix_in_raw_type() {
    // First parameter written as `struct foo *`, no typedef.
    let header = c_model(vec![], vec![class("foo", 1)], vec![]);
    let impl_model = c_model(vec![func("foo_bar", Some("struct foo *"))], vec![], vec![]);
    let mut models = vec![
        (PathBuf::from("foo.h"), header),
        (PathBuf::from("foo.c"), impl_model),
    ];
    enrich_c_oop(&mut models);
    assert_eq!(models[0].1.classes[0].method_count, 1);
}

#[test]
fn non_c_models_pass_through_unchanged() {
    let rust_model = SourceModel {
        language: "rust".into(),
        total_lines: 5,
        functions: vec![func("foo_bar", Some("foo *"))],
        classes: vec![class("foo", 1)],
        imports: vec![],
        comments: vec![],
        type_aliases: vec![],
    };
    let mut models = vec![(PathBuf::from("foo.rs"), rust_model)];
    models.push((
        PathBuf::from("bar.c"),
        c_model(vec![], vec![class("bar", 1)], vec![]),
    ));
    enrich_c_oop(&mut models);
    assert_eq!(models[0].1.classes[0].method_count, 0);
    assert!(!models[0].1.classes[0].has_behavior);
}

// ── is_exported tightening ──

#[test]
fn tightens_is_exported_when_not_declared_in_header() {
    let header = c_model(
        vec![func("public_api", Some("foo *"))],
        vec![class("foo", 1)],
        vec![],
    );
    let impl_model = c_model(
        vec![
            func("public_api", Some("foo *")),
            func("internal_helper", None),
        ],
        vec![],
        vec![],
    );
    let mut models = vec![
        (PathBuf::from("foo.h"), header),
        (PathBuf::from("foo.c"), impl_model),
    ];
    enrich_c_oop(&mut models);
    let impl_fns = &models[1].1.functions;
    assert!(
        impl_fns
            .iter()
            .find(|f| f.name == "public_api")
            .unwrap()
            .is_exported,
        "declared in header, stays exported"
    );
    assert!(
        !impl_fns
            .iter()
            .find(|f| f.name == "internal_helper")
            .unwrap()
            .is_exported,
        "not declared in any header, demoted to private"
    );
}

#[test]
fn leaves_header_functions_alone() {
    let header = c_model(vec![func("some_api", None)], vec![], vec![]);
    let mut models = vec![(PathBuf::from("api.h"), header)];
    enrich_c_oop(&mut models);
    assert!(models[0].1.functions[0].is_exported);
}
