#[cfg(test)]
mod plugin_tests {
    use std::path::PathBuf;

    use cha_core::plugins::*;
    use cha_core::*;

    fn make_file() -> SourceFile {
        SourceFile::new(PathBuf::from("test.rs"), String::new())
    }

    fn make_model(
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

    fn func(name: &str, lines: usize, complexity: usize, exported: bool) -> FunctionInfo {
        FunctionInfo {
            name: name.into(),
            start_line: 1,
            end_line: lines,
            line_count: lines,
            complexity,
            body_hash: Some(lines as u64),
            is_exported: exported,
        }
    }

    fn func_with_hash(name: &str, lines: usize, hash: u64) -> FunctionInfo {
        FunctionInfo {
            name: name.into(),
            start_line: 1,
            end_line: lines,
            line_count: lines,
            complexity: 1,
            body_hash: Some(hash),
            is_exported: false,
        }
    }

    fn class(name: &str, methods: usize, lines: usize, exported: bool) -> ClassInfo {
        ClassInfo {
            name: name.into(),
            start_line: 1,
            end_line: lines,
            method_count: methods,
            line_count: lines,
            is_exported: exported,
        }
    }

    fn import(source: &str, line: usize) -> ImportInfo {
        ImportInfo {
            source: source.into(),
            line,
        }
    }

    fn analyze(plugin: &dyn Plugin, model: &SourceModel) -> Vec<Finding> {
        let file = make_file();
        let ctx = AnalysisContext { file: &file, model };
        plugin.analyze(&ctx)
    }

    // -- LengthAnalyzer --

    #[test]
    fn length_long_method_triggers() {
        let model = make_model(vec![func("big", 31, 1, false)], vec![], vec![], 31);
        let findings = analyze(&LengthAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "long_method");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn length_at_threshold_no_trigger() {
        let model = make_model(vec![func("ok", 30, 1, false)], vec![], vec![], 30);
        let findings = analyze(&LengthAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn length_double_threshold_error() {
        let model = make_model(vec![func("huge", 61, 1, false)], vec![], vec![], 61);
        let findings = analyze(&LengthAnalyzer::default(), &model);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn length_large_class() {
        let model = make_model(vec![], vec![class("Big", 11, 201, false)], vec![], 201);
        let findings = analyze(&LengthAnalyzer::default(), &model);
        assert!(findings.iter().any(|f| f.smell_name == "large_class"));
    }

    // -- ComplexityAnalyzer --

    #[test]
    fn complexity_warning() {
        let model = make_model(vec![func("complex", 10, 10, false)], vec![], vec![], 10);
        let findings = analyze(&ComplexityAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn complexity_error() {
        let model = make_model(
            vec![func("very_complex", 10, 20, false)],
            vec![],
            vec![],
            10,
        );
        let findings = analyze(&ComplexityAnalyzer::default(), &model);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn complexity_below_threshold() {
        let model = make_model(vec![func("simple", 10, 9, false)], vec![], vec![], 10);
        let findings = analyze(&ComplexityAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    // -- DuplicateCodeAnalyzer --

    #[test]
    fn duplicate_triggers() {
        let model = make_model(
            vec![func_with_hash("a", 5, 42), func_with_hash("b", 5, 42)],
            vec![],
            vec![],
            10,
        );
        let findings = analyze(&DuplicateCodeAnalyzer, &model);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].smell_name, "duplicate_code");
    }

    #[test]
    fn duplicate_different_hash_no_trigger() {
        let model = make_model(
            vec![func_with_hash("a", 5, 42), func_with_hash("b", 5, 99)],
            vec![],
            vec![],
            10,
        );
        let findings = analyze(&DuplicateCodeAnalyzer, &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn duplicate_short_fn_ignored() {
        let model = make_model(
            vec![func_with_hash("a", 3, 42), func_with_hash("b", 3, 42)],
            vec![],
            vec![],
            6,
        );
        let findings = analyze(&DuplicateCodeAnalyzer, &model);
        assert!(findings.is_empty());
    }

    // -- CouplingAnalyzer --

    #[test]
    fn coupling_warning() {
        let imports: Vec<_> = (0..16)
            .map(|i| import(&format!("mod_{i}"), i + 1))
            .collect();
        let model = make_model(vec![], vec![], imports, 20);
        let findings = analyze(&CouplingAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn coupling_error_double() {
        let imports: Vec<_> = (0..31)
            .map(|i| import(&format!("mod_{i}"), i + 1))
            .collect();
        let model = make_model(vec![], vec![], imports, 40);
        let findings = analyze(&CouplingAnalyzer::default(), &model);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn coupling_at_threshold_no_trigger() {
        let imports: Vec<_> = (0..15)
            .map(|i| import(&format!("mod_{i}"), i + 1))
            .collect();
        let model = make_model(vec![], vec![], imports, 20);
        let findings = analyze(&CouplingAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    // -- NamingAnalyzer --

    #[test]
    fn naming_too_short() {
        let model = make_model(vec![func("x", 5, 1, false)], vec![], vec![], 5);
        let findings = analyze(&NamingAnalyzer::default(), &model);
        assert!(findings.iter().any(|f| f.smell_name == "naming_too_short"));
    }

    #[test]
    fn naming_class_lowercase() {
        let model = make_model(vec![], vec![class("myClass", 0, 5, false)], vec![], 5);
        let findings = analyze(&NamingAnalyzer::default(), &model);
        assert!(findings.iter().any(|f| f.smell_name == "naming_convention"));
    }

    #[test]
    fn naming_ok() {
        let model = make_model(
            vec![func("process_data", 5, 1, false)],
            vec![class("DataProcessor", 0, 5, false)],
            vec![],
            10,
        );
        let findings = analyze(&NamingAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    // -- DeadCodeAnalyzer --

    #[test]
    fn dead_code_unexported_unreferenced() {
        let file = SourceFile::new(
            PathBuf::from("test.rs"),
            "fn unused() {\n    todo!()\n}\n".into(),
        );
        let model = make_model(vec![func("unused", 3, 1, false)], vec![], vec![], 3);
        let ctx = AnalysisContext {
            file: &file,
            model: &model,
        };
        let findings = DeadCodeAnalyzer.analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "dead_code");
    }

    #[test]
    fn dead_code_exported_no_trigger() {
        let model = make_model(vec![func("public_fn", 3, 1, true)], vec![], vec![], 3);
        let findings = analyze(&DeadCodeAnalyzer, &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn dead_code_main_no_trigger() {
        let model = make_model(vec![func("main", 3, 1, false)], vec![], vec![], 3);
        let findings = analyze(&DeadCodeAnalyzer, &model);
        assert!(findings.is_empty());
    }

    // -- ApiSurfaceAnalyzer --

    #[test]
    fn api_surface_over_exposed() {
        let fns: Vec<_> = (0..5)
            .map(|i| func(&format!("fn_{i}"), 5, 1, true))
            .collect();
        let model = make_model(fns, vec![], vec![], 25);
        let findings = analyze(&ApiSurfaceAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "large_api_surface");
    }

    #[test]
    fn api_surface_below_min_items() {
        let fns: Vec<_> = (0..4)
            .map(|i| func(&format!("fn_{i}"), 5, 1, true))
            .collect();
        let model = make_model(fns, vec![], vec![], 20);
        let findings = analyze(&ApiSurfaceAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    // -- LayerViolationAnalyzer --

    #[test]
    fn layer_violation_triggers() {
        let analyzer = LayerViolationAnalyzer::from_config_str("domain:0,service:1,controller:2");
        let file = SourceFile::new(PathBuf::from("domain/repo.rs"), String::new());
        let model = make_model(vec![], vec![], vec![import("controller/handler", 1)], 5);
        let ctx = AnalysisContext {
            file: &file,
            model: &model,
        };
        let findings = analyzer.analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn layer_violation_same_layer_ok() {
        let analyzer = LayerViolationAnalyzer::from_config_str("domain:0,service:1");
        let file = SourceFile::new(PathBuf::from("service/a.rs"), String::new());
        let model = make_model(vec![], vec![], vec![import("service/b", 1)], 5);
        let ctx = AnalysisContext {
            file: &file,
            model: &model,
        };
        let findings = analyzer.analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn layer_violation_no_config_no_trigger() {
        let model = make_model(vec![], vec![], vec![import("anything", 1)], 5);
        let findings = analyze(&LayerViolationAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }
}
