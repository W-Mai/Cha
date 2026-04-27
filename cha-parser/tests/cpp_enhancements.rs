//! Architectural-constraint tests for C++ parser enhancements:
//! out-of-class method definitions (`ClassName::method()`), namespace
//! nesting, and templates. These cover roadmap items "C++ Parser
//! Enhancement" (§566-569).

use cha_core::SourceFile;
use cha_parser::{CppParser, LanguageParser};

fn parse(content: &str) -> cha_core::SourceModel {
    let file = SourceFile {
        path: std::path::PathBuf::from("t.cpp"),
        content: content.to_string(),
    };
    CppParser.parse(&file).unwrap()
}

#[test]
fn out_of_class_method_attributes_to_class() {
    let m = parse(
        "class Foo {\n\
         public:\n\
             void bar();\n\
         };\n\
         void Foo::bar() { return; }\n",
    );
    assert!(
        m.functions.iter().any(|f| f.name == "bar"),
        "expected function `bar` from out-of-class definition"
    );
    let foo = m.classes.iter().find(|c| c.name == "Foo").unwrap();
    assert!(
        foo.method_count >= 1,
        "expected Foo.method_count >= 1, got {}",
        foo.method_count
    );
    assert!(foo.has_behavior, "expected Foo.has_behavior = true");
}

#[test]
fn global_qualified_function_is_captured() {
    let m = parse("double ::global(int x) { return x * 2.0; }\n");
    assert!(
        m.functions.iter().any(|f| f.name == "global"),
        "expected `global` from ::global definition"
    );
}

#[test]
fn nested_namespace_functions_are_captured() {
    let m = parse("namespace a { namespace b { int f(int x) { return x + 1; } } }\n");
    assert!(
        m.functions.iter().any(|f| f.name == "f"),
        "nested namespace function should be extracted; got {:?}",
        m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

#[test]
fn templated_class_is_captured() {
    let m = parse(
        "template <typename T>\n\
         class Box { public: T value; T get() const { return value; } };\n",
    );
    assert!(
        m.classes.iter().any(|c| c.name == "Box"),
        "templated class should be extracted"
    );
}

#[test]
fn templated_free_function_is_captured() {
    let m = parse("template <typename T>\nT identity(T x) { return x; }\n");
    assert!(
        m.functions.iter().any(|f| f.name == "identity"),
        "templated free function should be extracted"
    );
}

#[test]
fn destructor_out_of_class_is_captured() {
    let m = parse(
        "class Foo { public: ~Foo(); };\n\
         Foo::~Foo() { }\n",
    );
    // Destructor name in tree-sitter-cpp is `destructor_name` which wraps
    // an `~Foo` identifier. Record it as a function so downstream analysis
    // doesn't silently drop it.
    assert!(
        m.functions
            .iter()
            .any(|f| f.name == "~Foo" || f.name.contains("Foo")),
        "destructor should be captured; got {:?}",
        m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

#[test]
fn operator_out_of_class_is_captured() {
    let m = parse(
        "class Foo { public: Foo operator+(const Foo&) const; };\n\
         Foo Foo::operator+(const Foo& rhs) const { return *this; }\n",
    );
    assert!(
        m.functions.iter().any(|f| f.name.contains("operator+")),
        "out-of-class operator+ should be captured; got {:?}",
        m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

#[test]
fn nested_class_scope_resolved() {
    // A::B::method — qualifier should be A::B, attribute to class B.
    let m = parse(
        "namespace A {\n\
             class B { public: void m(); };\n\
         }\n\
         void A::B::m() { return; }\n",
    );
    assert!(m.functions.iter().any(|f| f.name == "m"));
    let b = m
        .classes
        .iter()
        .find(|c| c.name == "B")
        .expect("class B should exist");
    assert!(b.method_count >= 1, "A::B::m should attribute to class B");
}
