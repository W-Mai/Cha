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
            parameter_count: 0,
            parameter_types: vec![],
            chain_depth: 0,
            switch_arms: 0,
            external_refs: vec![],
            is_delegating: false,
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
            parameter_count: 0,
            parameter_types: vec![],
            chain_depth: 0,
            switch_arms: 0,
            external_refs: vec![],
            is_delegating: false,
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
            delegating_method_count: 0,
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
        let model = make_model(vec![func("big", 51, 1, false)], vec![], vec![], 51);
        let findings = analyze(&LengthAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "long_method");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn length_at_threshold_no_trigger() {
        let model = make_model(vec![func("ok", 50, 1, false)], vec![], vec![], 50);
        let findings = analyze(&LengthAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn length_double_threshold_error() {
        let model = make_model(vec![func("huge", 101, 1, false)], vec![], vec![], 101);
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
            vec![func_with_hash("a", 15, 42), func_with_hash("b", 15, 42)],
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
            vec![func_with_hash("a", 15, 42), func_with_hash("b", 15, 99)],
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

    // -- LongParameterListAnalyzer --

    #[test]
    fn long_param_list_triggers() {
        let mut f = func("many_params", 10, 1, false);
        f.parameter_count = 6;
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&LongParameterListAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "long_parameter_list");
    }

    #[test]
    fn long_param_list_at_threshold() {
        let mut f = func("ok", 10, 1, false);
        f.parameter_count = 5;
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&LongParameterListAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn long_param_list_below() {
        let mut f = func("few", 10, 1, false);
        f.parameter_count = 2;
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&LongParameterListAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    // -- SwitchStatementAnalyzer --

    #[test]
    fn switch_statement_triggers() {
        let mut f = func("big_match", 20, 1, false);
        f.switch_arms = 9;
        let model = make_model(vec![f], vec![], vec![], 20);
        let findings = analyze(&SwitchStatementAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "switch_statement");
    }

    #[test]
    fn switch_statement_at_threshold() {
        let mut f = func("ok_match", 20, 1, false);
        f.switch_arms = 8;
        let model = make_model(vec![f], vec![], vec![], 20);
        let findings = analyze(&SwitchStatementAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn switch_statement_no_arms() {
        let model = make_model(vec![func("plain", 10, 1, false)], vec![], vec![], 10);
        let findings = analyze(&SwitchStatementAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    // -- MessageChainAnalyzer --

    #[test]
    fn message_chain_triggers() {
        let mut f = func("deep", 10, 1, false);
        f.chain_depth = 4;
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&MessageChainAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "message_chain");
    }

    #[test]
    fn message_chain_at_threshold() {
        let mut f = func("ok", 10, 1, false);
        f.chain_depth = 3;
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&MessageChainAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn message_chain_shallow() {
        let mut f = func("shallow", 10, 1, false);
        f.chain_depth = 1;
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&MessageChainAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    // -- PrimitiveObsessionAnalyzer --

    #[test]
    fn primitive_obsession_triggers() {
        let mut f = func("prim", 10, 1, false);
        f.parameter_types = vec!["i32".into(), "String".into(), "bool".into()];
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&PrimitiveObsessionAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "primitive_obsession");
    }

    #[test]
    fn primitive_obsession_mixed_types() {
        let mut f = func("mixed", 10, 1, false);
        f.parameter_types = vec!["i32".into(), "MyStruct".into(), "bool".into()];
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&PrimitiveObsessionAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn primitive_obsession_too_few_params() {
        let mut f = func("few", 10, 1, false);
        f.parameter_types = vec!["i32".into(), "bool".into()];
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&PrimitiveObsessionAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    // -- DataClumpsAnalyzer --

    #[test]
    fn data_clumps_triggers() {
        let sig = vec!["String".into(), "i32".into(), "bool".into()];
        let mk = |name| {
            let mut f = func(name, 10, 1, false);
            f.parameter_types = sig.clone();
            f
        };
        let model = make_model(vec![mk("a"), mk("b"), mk("c")], vec![], vec![], 30);
        let findings = analyze(&DataClumpsAnalyzer::default(), &model);
        assert_eq!(findings.len(), 3);
        assert_eq!(findings[0].smell_name, "data_clumps");
    }

    #[test]
    fn data_clumps_different_sigs() {
        let mut f1 = func("a", 10, 1, false);
        f1.parameter_types = vec!["i32".into(), "bool".into(), "String".into()];
        let mut f2 = func("b", 10, 1, false);
        f2.parameter_types = vec!["f64".into(), "Vec".into(), "Option".into()];
        let mut f3 = func("c", 10, 1, false);
        f3.parameter_types = vec!["u8".into(), "u16".into(), "u32".into()];
        let model = make_model(vec![f1, f2, f3], vec![], vec![], 30);
        let findings = analyze(&DataClumpsAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn data_clumps_below_min_occurrences() {
        let sig = vec!["i32".into(), "bool".into(), "String".into()];
        let mk = |name| {
            let mut f = func(name, 10, 1, false);
            f.parameter_types = sig.clone();
            f
        };
        let model = make_model(vec![mk("a"), mk("b")], vec![], vec![], 20);
        let findings = analyze(&DataClumpsAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    // -- FeatureEnvyAnalyzer --

    #[test]
    fn feature_envy_triggers() {
        let mut f = func("envious", 10, 1, false);
        f.external_refs = vec!["db".into(), "db".into(), "db".into()];
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&FeatureEnvyAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "feature_envy");
    }

    #[test]
    fn feature_envy_spread_refs() {
        let mut f = func("spread", 10, 1, false);
        f.external_refs = vec!["a".into(), "b".into(), "c".into()];
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&FeatureEnvyAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn feature_envy_too_few_refs() {
        let mut f = func("few", 10, 1, false);
        f.external_refs = vec!["db".into(), "db".into()];
        let model = make_model(vec![f], vec![], vec![], 10);
        let findings = analyze(&FeatureEnvyAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    // -- MiddleManAnalyzer --

    #[test]
    fn middle_man_triggers() {
        let c = ClassInfo {
            name: "Proxy".into(),
            start_line: 1,
            end_line: 10,
            method_count: 4,
            line_count: 10,
            is_exported: false,
            delegating_method_count: 3,
        };
        let model = make_model(vec![], vec![c], vec![], 10);
        let findings = analyze(&MiddleManAnalyzer::default(), &model);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].smell_name, "middle_man");
    }

    #[test]
    fn middle_man_below_ratio() {
        let c = ClassInfo {
            name: "Mixed".into(),
            start_line: 1,
            end_line: 10,
            method_count: 4,
            line_count: 10,
            is_exported: false,
            delegating_method_count: 1,
        };
        let model = make_model(vec![], vec![c], vec![], 10);
        let findings = analyze(&MiddleManAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }

    #[test]
    fn middle_man_too_few_methods() {
        let c = ClassInfo {
            name: "Tiny".into(),
            start_line: 1,
            end_line: 5,
            method_count: 2,
            line_count: 5,
            is_exported: false,
            delegating_method_count: 2,
        };
        let model = make_model(vec![], vec![c], vec![], 5);
        let findings = analyze(&MiddleManAnalyzer::default(), &model);
        assert!(findings.is_empty());
    }
}
