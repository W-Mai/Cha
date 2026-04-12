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
        parameter_count: 0,
        parameter_types: vec![],
        chain_depth: 0,
        switch_arms: 0,
        external_refs: vec![],
        is_delegating: false,
        comment_lines: 0,
        referenced_fields: vec![],
        null_check_fields: vec![],
        switch_dispatch_target: None,
        optional_param_count: 0,
        called_functions: Vec::new(),
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
        is_exported: false,
        parameter_count: 0,
        parameter_types: vec![],
        chain_depth: 0,
        switch_arms: 0,
        external_refs: vec![],
        is_delegating: false,
        comment_lines: 0,
        referenced_fields: vec![],
        null_check_fields: vec![],
        switch_dispatch_target: None,
        optional_param_count: 0,
        called_functions: Vec::new(),
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
        delegating_method_count: 0,
        field_count: 0,
        field_names: vec![],
        has_behavior: false,
        is_interface: false,
        parent_name: None,
        override_count: 0,
        self_call_count: 0,
        has_listener_field: false,
        has_notify_method: false,
    }
}

pub fn import(source: &str, line: usize) -> ImportInfo {
    ImportInfo {
        source: source.into(),
        line,
    }
}

pub fn analyze(plugin: &dyn Plugin, model: &SourceModel) -> Vec<Finding> {
    let file = make_file();
    let ctx = AnalysisContext { file: &file, model };
    plugin.analyze(&ctx)
}
