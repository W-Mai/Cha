use std::path::PathBuf;

use cha_core::plugins::*;
use cha_core::*;

pub fn make_file() -> SourceFile {
    SourceFile::new(PathBuf::from("test.rs"), String::new())
}

pub fn make_model(
    functions: Vec<FunctionInfo>,
    classes: Vec<ClassInfo>,
    imports: Vec<ImportInfo>,
    total_lines: usize,
) -> SourceModel {
    SourceModel {
        language: "rust".into(),
        total_lines,
        functions,
        classes,
        imports,
        comments: Vec::new(),
        type_aliases: Vec::new(),
    }
}

pub fn func(name: &str, lines: usize, complexity: usize, exported: bool) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        start_line: 1,
        end_line: lines,
        line_count: lines,
        complexity,
        body_hash: Some(lines as u64),
        is_exported: exported,
        ..Default::default()
    }
}

pub fn func_with_hash(name: &str, lines: usize, hash: u64) -> FunctionInfo {
    FunctionInfo {
        name: name.into(),
        start_line: 1,
        end_line: lines,
        line_count: lines,
        complexity: 1,
        body_hash: Some(hash),
        ..Default::default()
    }
}

pub fn class(name: &str, methods: usize, lines: usize, exported: bool) -> ClassInfo {
    ClassInfo {
        name: name.into(),
        start_line: 1,
        end_line: lines,
        method_count: methods,
        line_count: lines,
        is_exported: exported,
        ..Default::default()
    }
}

pub fn import(source: &str, line: usize) -> ImportInfo {
    ImportInfo {
        source: source.into(),
        line,
        ..Default::default()
    }
}

/// Shorthand: TypeRef carrying `name`, raw same as name, origin Unknown.
pub fn tref(name: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin: TypeOrigin::Unknown,
    }
}

/// TypeRef for a known primitive.
pub fn tref_prim(name: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: name.into(),
        origin: TypeOrigin::Primitive,
    }
}

/// TypeRef for an external module type.
pub fn tref_ext(name: &str, module: &str) -> TypeRef {
    TypeRef {
        name: name.into(),
        raw: format!("{module}::{name}"),
        origin: TypeOrigin::External(module.into()),
    }
}

pub fn analyze(plugin: &dyn Plugin, model: &SourceModel) -> Vec<Finding> {
    let file = make_file();
    let ctx = AnalysisContext { file: &file, model };
    plugin.analyze(&ctx)
}
