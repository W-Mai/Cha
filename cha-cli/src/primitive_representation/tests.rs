use super::*;
use cha_core::{FunctionInfo, SourceModel, TypeOrigin, TypeRef};
use std::path::PathBuf;

fn tref_primitive(name: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin: TypeOrigin::Primitive,
    }
}

fn tref_local(name: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin: TypeOrigin::Local,
    }
}

fn exported_fn(name: &str, params: Vec<(&str, TypeRef)>) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        is_exported: true,
        start_line: 1,
        end_line: 1,
        parameter_count: params.len(),
        parameter_types: params.iter().map(|(_, t)| t.clone()).collect(),
        parameter_names: params.iter().map(|(n, _)| (*n).to_string()).collect(),
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

fn run_on(m: SourceModel) -> Vec<Finding> {
    let idx = ProjectIndex::from_models(vec![(PathBuf::from("t.rs"), m)]);
    detect(&idx)
}

#[test]
fn param_named_id_with_i32_flags() {
    let m = model_with(vec![exported_fn(
        "get_user",
        vec![("user_id", tref_primitive("i32"))],
    )]);
    let findings = run_on(m);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("user_id"));
    assert!(findings[0].message.contains("i32"));
}

#[test]
fn param_named_email_with_string_flags() {
    let m = model_with(vec![exported_fn(
        "send",
        vec![("email", tref_primitive("String"))],
    )]);
    let findings = run_on(m);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("email"));
}

#[test]
fn param_named_x_with_i32_quiet() {
    let m = model_with(vec![exported_fn(
        "add",
        vec![("x", tref_primitive("i32")), ("y", tref_primitive("i32"))],
    )]);
    assert!(
        run_on(m).is_empty(),
        "pure-math param names should not fire"
    );
}

#[test]
fn param_named_id_with_domain_type_quiet() {
    let m = model_with(vec![exported_fn(
        "get_user",
        vec![("id", tref_local("UserId"))],
    )]);
    assert!(
        run_on(m).is_empty(),
        "param typed as a local newtype should not fire even if named `id`"
    );
}

#[test]
fn business_token_substring_quiet() {
    // "widget_identifier" tokens to [widget, identifier] — "id" is a
    // prefix of "identifier" but not a standalone token. Must not fire.
    let m = model_with(vec![exported_fn(
        "paint",
        vec![("widget_identifier", tref_primitive("i32"))],
    )]);
    assert!(
        run_on(m).is_empty(),
        "substring `id` inside `identifier` should not trigger"
    );
}

#[test]
fn multiple_primitives_merged_into_one_finding() {
    let m = model_with(vec![exported_fn(
        "login",
        vec![
            ("email", tref_primitive("String")),
            ("password", tref_primitive("String")),
        ],
    )]);
    let findings = run_on(m);
    assert_eq!(findings.len(), 1, "one finding per function, not per param");
    assert!(findings[0].message.contains("email"));
    assert!(findings[0].message.contains("password"));
    assert_eq!(findings[0].actual_value, Some(2.0));
}

#[test]
fn noise_name_overrides_business_match() {
    // "count" is in both lists on purpose — a raw `count: i32` is not
    // a Primitive Representation smell. Noise wins.
    let m = model_with(vec![exported_fn(
        "iterate",
        vec![("count", tref_primitive("i32"))],
    )]);
    assert!(run_on(m).is_empty());
}

#[test]
fn non_exported_function_quiet() {
    let mut f = exported_fn("internal_helper", vec![("email", tref_primitive("String"))]);
    f.is_exported = false;
    let m = model_with(vec![f]);
    assert!(
        run_on(m).is_empty(),
        "private helpers are noise — design signal applies to public surface"
    );
}

#[test]
fn camel_case_business_name_flags() {
    // TS / Java style camelCase
    let m = model_with(vec![exported_fn(
        "getUser",
        vec![("userId", tref_primitive("number"))],
    )]);
    let findings = run_on(m);
    assert_eq!(findings.len(), 1);
}

#[test]
fn container_typed_param_quiet() {
    // `Path` / `PathBuf` / `Vec<T>` etc. are already domain-carrying
    // types in their own right — wrapping `path: &Path` in a newtype
    // would destroy the abstraction rather than preserve one.
    let m = model_with(vec![exported_fn(
        "load",
        vec![
            ("path", tref_primitive("Path")),
            ("items", tref_primitive("Vec")),
        ],
    )]);
    assert!(
        run_on(m).is_empty(),
        "container types should not trigger even with business-y param names"
    );
}

#[test]
fn acronym_http_url_flags() {
    // "apiUrl" → [api, url] — url is in BUSINESS_TOKENS
    let m = model_with(vec![exported_fn(
        "fetch",
        vec![("apiUrl", tref_primitive("String"))],
    )]);
    let findings = run_on(m);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("apiUrl"));
}
