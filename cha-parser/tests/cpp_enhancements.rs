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
fn reference_return_type_does_not_drop_function() {
    // Regression: `reference_declarator` (e.g. `const int& Foo::get()`)
    // lacks a named `declarator` field, which previously made the
    // declarator walker return None and the function vanish.
    let m = parse(
        "class Foo { public: const int& get(); };\n\
         const int& Foo::get() { static int x = 0; return x; }\n",
    );
    assert!(
        m.functions.iter().any(|f| f.name == "get"),
        "reference-return method should still be captured; got {:?}",
        m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    let foo = m.classes.iter().find(|c| c.name == "Foo").unwrap();
    assert!(foo.method_count >= 1, "Foo::get should still attribute");
}

#[test]
fn pointer_return_type_captured() {
    // `pointer_declarator` has a `declarator` field; make sure the happy
    // path still works alongside the reference fix.
    let m = parse(
        "class Foo { public: int* ptr(); };\n\
         int* Foo::ptr() { return nullptr; }\n",
    );
    assert!(m.functions.iter().any(|f| f.name == "ptr"));
    let foo = m.classes.iter().find(|c| c.name == "Foo").unwrap();
    assert!(foo.method_count >= 1);
}

#[test]
fn multiple_out_of_class_methods_attribute_to_same_class() {
    let m = parse(
        "class Foo { public: void a(); void b(); void c(); };\n\
         void Foo::a() {}\n\
         void Foo::b() {}\n\
         void Foo::c() {}\n",
    );
    assert_eq!(m.functions.len(), 3);
    let foo = m.classes.iter().find(|c| c.name == "Foo").unwrap();
    // 3 declarations inside the class body + 3 out-of-class definitions.
    assert!(
        foo.method_count >= 3,
        "all three methods should attribute; got {}",
        foo.method_count
    );
}

#[test]
fn constructor_out_of_class_captured() {
    let m = parse(
        "class Foo { public: Foo(int); };\n\
         Foo::Foo(int x) { (void)x; }\n",
    );
    assert!(
        m.functions.iter().any(|f| f.name == "Foo"),
        "constructor should be captured as a function"
    );
}

#[test]
fn extern_c_linkage_functions_captured() {
    let m = parse(
        "extern \"C\" {\n\
             int foo(int x) { return x + 1; }\n\
             int bar(int y) { return y * 2; }\n\
         }\n",
    );
    assert!(m.functions.iter().any(|f| f.name == "foo"));
    assert!(m.functions.iter().any(|f| f.name == "bar"));
}

#[test]
fn const_member_method_out_of_class() {
    let m = parse(
        "class Foo { public: int bar() const; };\n\
         int Foo::bar() const { return 0; }\n",
    );
    assert!(m.functions.iter().any(|f| f.name == "bar"));
    let foo = m.classes.iter().find(|c| c.name == "Foo").unwrap();
    assert!(foo.method_count >= 1);
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
