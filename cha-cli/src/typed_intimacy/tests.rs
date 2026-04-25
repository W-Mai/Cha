use super::*;
use cha_core::{ClassInfo, SourceModel, TypeOrigin, TypeRef};

fn tref(name: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin: TypeOrigin::Local,
    }
}

fn class(name: &str) -> ClassInfo {
    ClassInfo {
        name: name.into(),
        start_line: 1,
        end_line: 5,
        ..Default::default()
    }
}

fn func_sig(name: &str, params: Vec<TypeRef>, ret: Option<TypeRef>) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        start_line: 1,
        end_line: 1,
        parameter_count: params.len(),
        parameter_types: params,
        return_type: ret,
        ..Default::default()
    }
}

fn model_with(functions: Vec<FunctionInfo>, classes: Vec<ClassInfo>) -> SourceModel {
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

#[test]
fn flags_bidirectional_type_flow() {
    // order.rs declares Order + takes a Customer in a function.
    // customer.rs declares Customer + takes an Order in a function.
    let order_file = model_with(
        vec![func_sig("attach_customer", vec![tref("Customer")], None)],
        vec![class("Order")],
    );
    let customer_file = model_with(
        vec![func_sig("last_order", vec![], Some(tref("Order")))],
        vec![class("Customer")],
    );
    let models = vec![
        (PathBuf::from("src/order.rs"), order_file),
        (PathBuf::from("src/customer.rs"), customer_file),
    ];
    let findings = detect(&ProjectIndex::from_models(models));
    assert_eq!(
        findings.len(),
        2,
        "one finding per side of the intimate pair"
    );
    assert!(findings.iter().all(|f| f.smell_name == "typed_intimacy"));
    assert!(findings[0].message.contains("Order") || findings[0].message.contains("Customer"));
}

#[test]
fn ignores_one_way_dependency() {
    // order.rs uses Customer but customer.rs never mentions Order.
    let order_file = model_with(
        vec![func_sig("attach", vec![tref("Customer")], None)],
        vec![class("Order")],
    );
    let customer_file = model_with(vec![], vec![class("Customer")]);
    let models = vec![
        (PathBuf::from("src/order.rs"), order_file),
        (PathBuf::from("src/customer.rs"), customer_file),
    ];
    let findings = detect(&ProjectIndex::from_models(models));
    assert!(findings.is_empty());
}

#[test]
fn ignores_same_file_self_use() {
    // A file referencing its own class in its own functions is fine — no
    // second file involved, can't be intimate with itself.
    let only_file = model_with(
        vec![func_sig("build", vec![], Some(tref("Widget")))],
        vec![class("Widget")],
    );
    let models = vec![(PathBuf::from("src/widget.rs"), only_file)];
    let findings = detect(&ProjectIndex::from_models(models));
    assert!(findings.is_empty());
}

#[test]
fn emits_per_pair_not_per_usage() {
    // File A uses 3 of B's classes, B uses 2 of A's classes — still one
    // pair, two findings (one per side), not six.
    let a = model_with(
        vec![
            func_sig("f1", vec![tref("B1")], None),
            func_sig("f2", vec![tref("B2")], None),
            func_sig("f3", vec![], Some(tref("B3"))),
        ],
        vec![class("A1"), class("A2")],
    );
    let b = model_with(
        vec![
            func_sig("g1", vec![tref("A1")], None),
            func_sig("g2", vec![], Some(tref("A2"))),
        ],
        vec![class("B1"), class("B2"), class("B3")],
    );
    let models = vec![
        (PathBuf::from("src/a.rs"), a),
        (PathBuf::from("src/b.rs"), b),
    ];
    let findings = detect(&ProjectIndex::from_models(models));
    assert_eq!(findings.len(), 2);
}
