use std::path::PathBuf;

use cha_core::SourceFile;
use cha_parser::parse_file;

fn fixture(name: &str) -> SourceFile {
    let path = PathBuf::from(format!("tests/fixtures/{name}"));
    let content = std::fs::read_to_string(&path).unwrap();
    SourceFile::new(path, content)
}

// -- Rust fixtures --

#[test]
fn rust_simple_fn() {
    let model = parse_file(&fixture("simple.rs")).unwrap();
    assert_eq!(model.language, "rust");
    assert_eq!(model.total_lines, 3);
    assert_eq!(model.functions.len(), 1);
    assert_eq!(model.functions[0].name, "hello");
    assert_eq!(model.functions[0].line_count, 3);
    assert_eq!(model.functions[0].complexity, 1);
    assert_eq!(model.classes.len(), 0);
    assert_eq!(model.imports.len(), 0);
}

#[test]
fn rust_complex_fn() {
    let model = parse_file(&fixture("complex.rs")).unwrap();
    assert_eq!(model.functions.len(), 1);
    let f = &model.functions[0];
    assert_eq!(f.name, "decide");
    assert_eq!(f.line_count, 17);
    // if + if + else + else if + match_arm x3 + base = 10
    assert_eq!(f.complexity, 10);
}

#[test]
fn rust_structs_and_impl() {
    let model = parse_file(&fixture("structs.rs")).unwrap();
    assert_eq!(model.classes.len(), 1);
    assert_eq!(model.classes[0].name, "Point");
    // impl methods
    assert_eq!(model.functions.len(), 2);
    assert_eq!(model.functions[0].name, "new");
    assert_eq!(model.functions[1].name, "distance");
}

#[test]
fn rust_imports() {
    let model = parse_file(&fixture("imports.rs")).unwrap();
    assert_eq!(model.imports.len(), 3);
    assert_eq!(model.imports[0].source, "std::collections::HashMap");
    assert_eq!(model.imports[1].source, "std::path::PathBuf");
    assert_eq!(model.imports[2].source, "std::io::Read");
    assert_eq!(model.imports[0].line, 1);
}

#[test]
fn rust_exports() {
    let model = parse_file(&fixture("exports.rs")).unwrap();
    assert_eq!(model.functions.len(), 2);
    assert!(model.functions[0].is_exported); // pub fn
    assert!(!model.functions[1].is_exported); // fn
    assert_eq!(model.classes.len(), 2);
    assert!(model.classes[0].is_exported); // pub struct
    assert!(!model.classes[1].is_exported); // struct
}

// -- TypeScript fixtures --

#[test]
fn ts_simple() {
    let model = parse_file(&fixture("simple.ts")).unwrap();
    assert_eq!(model.language, "typescript");
    assert_eq!(model.total_lines, 18);
    assert_eq!(model.functions.len(), 1);
    assert_eq!(model.functions[0].name, "greet");
    assert_eq!(model.classes.len(), 1);
    assert_eq!(model.classes[0].name, "Greeter");
    assert_eq!(model.classes[0].method_count, 2);
    assert_eq!(model.imports.len(), 2);
}

#[test]
fn tsx_exports() {
    let model = parse_file(&fixture("exports.tsx")).unwrap();
    assert_eq!(model.functions.len(), 2);
    assert!(model.functions[0].is_exported); // export function
    assert!(!model.functions[1].is_exported); // function
    assert_eq!(model.classes.len(), 2);
    assert!(model.classes[0].is_exported); // export class
    assert!(!model.classes[1].is_exported); // class
}

#[test]
fn unsupported_extension() {
    let file = SourceFile::new(PathBuf::from("test.rb"), "def foo; end".into());
    assert!(parse_file(&file).is_none());
}

// -- Python fixtures --

#[test]
fn python_simple() {
    let model = parse_file(&fixture("simple.py")).unwrap();
    assert_eq!(model.language, "python");
    assert_eq!(model.functions.len(), 13); // 4 top-level + 4 Animal + 3 Dog + 2 EmptyInterface
    let top_fn = model
        .functions
        .iter()
        .find(|f| f.name == "simple_function")
        .unwrap();
    assert_eq!(top_fn.parameter_count, 2);
    assert_eq!(top_fn.complexity, 1);

    let complex_fn = model
        .functions
        .iter()
        .find(|f| f.name == "complex_function")
        .unwrap();
    assert!(complex_fn.complexity >= 4); // for + if + elif + base
    assert_eq!(complex_fn.optional_param_count, 1);
    assert!(complex_fn.comment_lines >= 1);
}

#[test]
fn python_classes() {
    let model = parse_file(&fixture("simple.py")).unwrap();
    assert_eq!(model.classes.len(), 3); // Animal, Dog, EmptyInterface

    let animal = model.classes.iter().find(|c| c.name == "Animal").unwrap();
    assert_eq!(animal.method_count, 4);
    assert!(animal.field_count >= 2); // name, sound, listeners
    assert!(animal.has_listener_field);
    assert!(animal.has_notify_method);
    assert!(animal.has_behavior);

    let dog = model.classes.iter().find(|c| c.name == "Dog").unwrap();
    assert_eq!(dog.parent_name.as_deref(), Some("Animal"));
    assert!(dog.has_behavior);

    let iface = model
        .classes
        .iter()
        .find(|c| c.name == "EmptyInterface")
        .unwrap();
    assert!(iface.is_interface);
}

#[test]
fn python_imports() {
    let model = parse_file(&fixture("simple.py")).unwrap();
    assert!(model.imports.len() >= 4); // os, sys, pathlib.Path, typing.List, typing.Optional
    assert!(model.imports.iter().any(|i| i.source == "os"));
    assert!(model.imports.iter().any(|i| i.source.contains("Path")));
}

#[test]
fn python_chain_depth() {
    let model = parse_file(&fixture("simple.py")).unwrap();
    let chain_fn = model
        .functions
        .iter()
        .find(|f| f.name == "long_chain_example")
        .unwrap();
    assert!(chain_fn.chain_depth >= 4);
}

#[test]
fn python_delegating() {
    let model = parse_file(&fixture("simple.py")).unwrap();
    let del_fn = model
        .functions
        .iter()
        .find(|f| f.name == "delegating")
        .unwrap();
    assert!(del_fn.is_delegating);
}

// -- Go fixtures --

#[test]
fn go_simple() {
    let model = parse_file(&fixture("simple.go")).unwrap();
    assert_eq!(model.language, "go");
    assert_eq!(model.functions.len(), 2);
    assert_eq!(model.functions[0].name, "Hello");
    assert!(model.functions[0].is_exported);
    assert_eq!(model.functions[1].name, "add");
    assert!(!model.functions[1].is_exported);
    assert_eq!(model.imports.len(), 1);
}

#[test]
fn go_structs() {
    let model = parse_file(&fixture("structs.go")).unwrap();
    assert!(model.classes.len() >= 2);
    let server = model.classes.iter().find(|c| c.name == "Server").unwrap();
    assert!(server.is_exported);
    assert_eq!(server.field_count, 2);
    let handler = model.classes.iter().find(|c| c.name == "Handler").unwrap();
    assert!(handler.is_interface);
    // method declaration
    assert!(model.functions.iter().any(|f| f.name == "Start"));
}

#[test]
fn go_complex() {
    let model = parse_file(&fixture("complex.go")).unwrap();
    let f = &model.functions[0];
    assert_eq!(f.name, "Complex");
    assert!(f.complexity > 1);
    assert!(f.switch_arms >= 4);
}

// -- C fixtures --

#[test]
fn c_simple() {
    let model = parse_file(&fixture("simple.c")).unwrap();
    assert_eq!(model.language, "c");
    assert_eq!(model.functions.len(), 2);
    assert_eq!(model.functions[0].name, "add");
    assert_eq!(model.functions[0].parameter_count, 2);
    assert_eq!(model.imports.len(), 2);
    // typedef struct + named struct
    assert!(
        model.classes.len() >= 2,
        "expected >= 2 structs, got {}",
        model.classes.len()
    );
    assert!(
        model.classes.iter().any(|c| c.name == "Point"),
        "missing typedef struct Point"
    );
    assert!(
        model.classes.iter().any(|c| c.name == "Color"),
        "missing struct Color"
    );
}

// -- C++ fixtures --

#[test]
fn cpp_simple() {
    let model = parse_file(&fixture("simple.cpp")).unwrap();
    assert_eq!(model.language, "cpp");
    assert!(model.functions.len() >= 1);
    let factorial = model
        .functions
        .iter()
        .find(|f| f.name == "factorial")
        .unwrap();
    assert!(factorial.complexity > 1);
    assert!(model.classes.len() >= 1);
    let animal = model.classes.iter().find(|c| c.name == "Animal").unwrap();
    assert!(animal.field_count >= 2);
}

// -- Cognitive complexity --

#[test]
fn go_cognitive_complexity() {
    let model = parse_file(&fixture("cognitive.go")).unwrap();
    let f = model
        .functions
        .iter()
        .find(|f| f.name == "SumOfPrimes")
        .unwrap();
    // for(+1) + for(+2,nest=1) + if(+3,nest=2) + continue_label(+1) = 7
    assert!(
        f.cognitive_complexity >= 5,
        "expected cognitive complexity >= 5, got {}",
        f.cognitive_complexity
    );
    assert!(
        f.cognitive_complexity > f.complexity || f.cognitive_complexity == f.complexity,
        "cognitive should generally be >= cyclomatic for nested code"
    );
}

// -- Cognitive complexity --
