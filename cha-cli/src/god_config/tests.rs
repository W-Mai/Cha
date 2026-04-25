use super::*;
use cha_core::{ClassInfo, FunctionInfo, SourceModel, TypeOrigin, TypeRef};

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

fn class(name: &str) -> ClassInfo {
    ClassInfo {
        name: name.into(),
        start_line: 1,
        end_line: 5,
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
fn flags_config_threaded_through_many_functions() {
    // 10 functions across 3 files all take AppConfig — qualifies as god.
    let cfg = tref("AppConfig");
    let mut a_fns = Vec::new();
    for i in 0..4 {
        a_fns.push(func(&format!("use_a_{i}"), vec![cfg.clone()]));
    }
    let a = model_with(a_fns, vec![class("AppConfig")]);
    let mut b_fns = Vec::new();
    for i in 0..4 {
        b_fns.push(func(&format!("use_b_{i}"), vec![cfg.clone()]));
    }
    let b = model_with(b_fns, vec![]);
    let mut c_fns = Vec::new();
    for i in 0..2 {
        c_fns.push(func(&format!("use_c_{i}"), vec![cfg.clone()]));
    }
    let c = model_with(c_fns, vec![]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("src/a.rs"), a),
        (PathBuf::from("src/b.rs"), b),
        (PathBuf::from("src/c.rs"), c),
    ]);
    let findings = detect(&idx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "god_config");
    assert!(findings[0].message.contains("AppConfig"));
    assert!(findings[0].message.contains("10 functions"));
    assert!(findings[0].message.contains("3 files"));
    assert_eq!(
        findings[0].location.path,
        PathBuf::from("src/a.rs"),
        "finding anchors to the file that declares the config"
    );
}

#[test]
fn ignores_under_thresholds() {
    // Each case would be a god_config if it cleared both thresholds, but
    // is rejected because exactly one does not:
    //   - Settings: 5 callers, 1 file → caller count too low.
    //   - Options: 10 callers, 1 file → file count too low (localised).
    let cases: [(&str, usize); 2] = [("Settings", 5), ("Options", 10)];
    for (type_name, n) in cases {
        let cfg = tref(type_name);
        let fns: Vec<FunctionInfo> = (0..n)
            .map(|i| func(&format!("f{i}"), vec![cfg.clone()]))
            .collect();
        let m = model_with(fns, vec![class(type_name)]);
        let idx = ProjectIndex::from_models(vec![(PathBuf::from("src/a.rs"), m)]);
        let findings = detect(&idx);
        assert!(findings.is_empty(), "case `{type_name}` should not fire");
    }
}

#[test]
fn ignores_non_config_type() {
    // 10 functions take User — not config-shaped.
    let user = tref("User");
    let mut fns = Vec::new();
    for i in 0..10 {
        fns.push(func(&format!("f{i}"), vec![user.clone()]));
    }
    let a = model_with(fns[0..4].to_vec(), vec![class("User")]);
    let b = model_with(fns[4..8].to_vec(), vec![]);
    let c = model_with(fns[8..10].to_vec(), vec![]);
    let idx = ProjectIndex::from_models(vec![
        (PathBuf::from("src/a.rs"), a),
        (PathBuf::from("src/b.rs"), b),
        (PathBuf::from("src/c.rs"), c),
    ]);
    let findings = detect(&idx);
    assert!(findings.is_empty());
}

#[test]
fn detects_suffix_pattern() {
    // `DatabaseConfig` ends in `Config` — matches suffix rule.
    let cfg = tref("DatabaseConfig");
    let mut files: Vec<(PathBuf, SourceModel)> = Vec::new();
    let mut decl_added = false;
    for f in 0..3 {
        let classes = if !decl_added {
            decl_added = true;
            vec![class("DatabaseConfig")]
        } else {
            vec![]
        };
        let fns: Vec<FunctionInfo> = (0..4)
            .map(|i| func(&format!("file{f}_fn{i}"), vec![cfg.clone()]))
            .collect();
        files.push((
            PathBuf::from(format!("src/f{f}.rs")),
            model_with(fns, classes),
        ));
    }
    let idx = ProjectIndex::from_models(files);
    let findings = detect(&idx);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("DatabaseConfig"));
}
