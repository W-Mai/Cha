use proptest::prelude::*;
use std::path::PathBuf;

use crate::model::{ClassInfo, FunctionInfo, ImportInfo};
use crate::{AnalysisContext, Finding, Plugin, SourceFile, SourceModel, plugins::*, reporter::*};

// -- Arbitrary strategies --

fn arb_function_info() -> impl Strategy<Value = FunctionInfo> {
    (
        "[a-z_][a-z0-9_]{0,20}",
        1..500usize,
        1..200usize,
        0..50usize,
        any::<Option<u64>>(),
        any::<bool>(),
    )
        .prop_map(
            |(name, start, lines, complexity, hash, exported)| FunctionInfo {
                name,
                start_line: start,
                end_line: start + lines,
                line_count: lines,
                complexity,
                body_hash: hash,
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
                cognitive_complexity: 0,
                ..Default::default()
            },
        )
}

fn arb_class_info() -> impl Strategy<Value = ClassInfo> {
    (
        "[A-Z][a-zA-Z0-9]{0,20}",
        1..500usize,
        1..500usize,
        0..30usize,
        any::<bool>(),
    )
        .prop_map(|(name, start, lines, methods, exported)| ClassInfo {
            name,
            start_line: start,
            end_line: start + lines,
            method_count: methods,
            line_count: lines,
            is_exported: exported,
            delegating_method_count: 0,
            field_count: 0,
            field_names: vec![],
            field_types: vec![],
            has_behavior: false,
            is_interface: false,
            parent_name: None,
            override_count: 0,
            self_call_count: 0,
            has_listener_field: false,
            has_notify_method: false,
            ..Default::default()
        })
}

fn arb_import_info() -> impl Strategy<Value = ImportInfo> {
    ("[a-z_/]{1,30}", 1..500usize).prop_map(|(source, line)| ImportInfo {
        source,
        line,
        ..Default::default()
    })
}

fn arb_source_model() -> impl Strategy<Value = SourceModel> {
    (
        prop::collection::vec(arb_function_info(), 0..10),
        prop::collection::vec(arb_class_info(), 0..5),
        prop::collection::vec(arb_import_info(), 0..20),
        1..1000usize,
    )
        .prop_map(|(functions, classes, imports, total_lines)| SourceModel {
            language: "rust".into(),
            total_lines,
            functions,
            classes,
            imports,
            comments: Vec::new(),
            type_aliases: Vec::new(),
        })
}

fn arb_finding() -> impl Strategy<Value = Finding> {
    (
        "[a-z_]{3,20}",
        arb_smell_category(),
        arb_severity(),
        ".{1,50}",
    )
        .prop_map(|(smell_name, category, severity, message)| Finding {
            smell_name,
            category,
            severity,
            location: default_test_location(),
            message,
            suggested_refactorings: vec!["Extract Method".into()],
            ..Default::default()
        })
}

fn arb_smell_category() -> impl Strategy<Value = crate::SmellCategory> {
    prop::sample::select(vec![
        crate::SmellCategory::Bloaters,
        crate::SmellCategory::OoAbusers,
        crate::SmellCategory::ChangePreventers,
        crate::SmellCategory::Dispensables,
        crate::SmellCategory::Couplers,
    ])
}

fn arb_severity() -> impl Strategy<Value = crate::Severity> {
    prop::sample::select(vec![
        crate::Severity::Hint,
        crate::Severity::Warning,
        crate::Severity::Error,
    ])
}

fn default_test_location() -> crate::Location {
    crate::Location {
        path: PathBuf::from("test.rs"),
        start_line: 1,
        start_col: 0,
        end_line: 10,
        end_col: 0,
        name: Some("test".into()),
    }
}

// -- Plugin property tests --

fn all_plugins() -> Vec<Box<dyn Plugin>> {
    vec![
        Box::new(LengthAnalyzer::default()),
        Box::new(ComplexityAnalyzer::default()),
        Box::new(DuplicateCodeAnalyzer),
        Box::new(CouplingAnalyzer::default()),
        Box::new(NamingAnalyzer::default()),
        Box::new(DeadCodeAnalyzer),
        Box::new(ApiSurfaceAnalyzer::default()),
        Box::new(LayerViolationAnalyzer::default()),
    ]
}

proptest! {
    #[test]
    fn plugins_never_panic(model in arb_source_model()) {
        let content = "fn main() {}\n".repeat(model.total_lines.max(1));
        let file = SourceFile::new(PathBuf::from("test.rs"), content);
        let ctx = AnalysisContext { file: &file, model: &model };

        for plugin in all_plugins() {
            let findings = plugin.analyze(&ctx);
            for f in &findings {
                // Severity is always valid (enforced by enum)
                prop_assert!(f.location.start_line > 0);
                prop_assert!(!f.smell_name.is_empty());
                prop_assert!(!f.message.is_empty());
            }
        }
    }

    #[test]
    fn plugin_finding_count_bounded(model in arb_source_model()) {
        let content = "fn main() {}\n".repeat(model.total_lines.max(1));
        let file = SourceFile::new(PathBuf::from("test.rs"), content);
        let ctx = AnalysisContext { file: &file, model: &model };

        let max_items = model.functions.len() + model.classes.len() + 1;
        let length = LengthAnalyzer::default();
        let findings = length.analyze(&ctx);
        // At most one finding per function + one per class + one for file
        prop_assert!(findings.len() <= max_items);
    }

    // -- Reporter property tests --

    #[test]
    fn terminal_reporter_never_panics(findings in prop::collection::vec(arb_finding(), 0..20)) {
        let reporter = TerminalReporter { show_all: true };
        let _ = reporter.render(&findings);
    }

    #[test]
    fn json_reporter_roundtrip(findings in prop::collection::vec(arb_finding(), 0..20)) {
        let reporter = JsonReporter;
        let json = reporter.render(&findings);
        let parsed: Vec<Finding> = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.len(), findings.len());
    }

    #[test]
    fn llm_reporter_never_panics(findings in prop::collection::vec(arb_finding(), 0..20)) {
        let reporter = LlmContextReporter;
        let _ = reporter.render(&findings);
    }

    #[test]
    fn sarif_reporter_valid_json(findings in prop::collection::vec(arb_finding(), 0..20)) {
        let reporter = SarifReporter;
        let sarif = reporter.render(&findings);
        let parsed: serde_json::Value = serde_json::from_str(&sarif).unwrap();
        prop_assert!(parsed["version"].as_str() == Some("2.1.0"));
    }
}
