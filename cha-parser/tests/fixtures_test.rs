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
    let file = SourceFile::new(PathBuf::from("test.py"), "def foo(): pass".into());
    assert!(parse_file(&file).is_none());
}
