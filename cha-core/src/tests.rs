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
        let reporter = TerminalReporter { show_all: true, top: None };
        let _ = reporter.render(&findings);
    }

    #[test]
    fn terminal_reporter_top_n_limits_output(findings in prop::collection::vec(arb_finding(), 5..20)) {
        let n = 3;
        let reporter = TerminalReporter { show_all: false, top: Some(n) };
        let out = reporter.render(&findings);
        let total_msg = format!("{} issue(s)", findings.len());
        let top_msg = format!("(showing top {n})");
        // summary shows total not n
        prop_assert!(out.contains(&total_msg));
        // "(showing top N)" suffix present when n < total
        prop_assert!(out.contains(&top_msg));
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

// -- prioritize_findings: concrete ordering behaviour --

#[cfg(test)]
mod prioritize_tests {
    use super::*;
    use crate::{Location, Severity, SmellCategory, prioritize_findings};

    fn finding_at(
        path: &str,
        smell: &str,
        severity: Severity,
        actual: Option<f64>,
        threshold: Option<f64>,
    ) -> Finding {
        Finding {
            smell_name: smell.into(),
            category: SmellCategory::Bloaters,
            severity,
            location: Location {
                path: PathBuf::from(path),
                start_line: 1,
                end_line: 1,
                ..Default::default()
            },
            message: "test".into(),
            actual_value: actual,
            threshold,
            ..Default::default()
        }
    }

    #[test]
    fn writes_risk_score_to_every_finding() {
        let mut findings = vec![
            finding_at("a.rs", "x", Severity::Hint, None, None),
            finding_at("b.rs", "y", Severity::Warning, Some(20.0), Some(10.0)),
        ];
        prioritize_findings(&mut findings);
        assert!(
            findings.iter().all(|f| f.risk_score.is_some()),
            "risk_score must be populated on every finding after prioritize"
        );
    }

    #[test]
    fn severity_outranks_overshoot_when_overshoot_is_tied() {
        // Two findings at overshoot 1 × compound 1 — the error-severity one
        // must come first.
        let mut findings = vec![
            finding_at("hint.rs", "a", Severity::Hint, None, None),
            finding_at("error.rs", "b", Severity::Error, None, None),
        ];
        prioritize_findings(&mut findings);
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[1].severity, Severity::Hint);
    }

    #[test]
    fn big_overshoot_beats_bare_severity_bump() {
        // Hint with actual/threshold = 5 → overshoot 5, score = 1×5 = 5.
        // Warning with no measurement → overshoot 1, score = 2×1 = 2.
        // The compound 5× problem should rank above the bare warning.
        let mut findings = vec![
            finding_at("warn.rs", "plain_warn", Severity::Warning, None, None),
            finding_at(
                "hint.rs",
                "big_overshoot",
                Severity::Hint,
                Some(500.0),
                Some(100.0),
            ),
        ];
        prioritize_findings(&mut findings);
        assert_eq!(
            findings[0].smell_name, "big_overshoot",
            "a 5× overshoot at hint severity should outrank a bare warning"
        );
    }

    #[test]
    fn hotspot_file_gets_compound_bonus() {
        // Four findings on hot.rs (> 3 → compound 1.5×) vs one finding on
        // cold.rs. With identical severity/overshoot the hot.rs findings
        // should all sort ahead of cold.rs.
        let mut findings = Vec::new();
        for i in 0..4 {
            findings.push(finding_at(
                "hot.rs",
                &format!("h{i}"),
                Severity::Warning,
                None,
                None,
            ));
        }
        findings.push(finding_at("cold.rs", "c0", Severity::Warning, None, None));
        prioritize_findings(&mut findings);
        assert_eq!(findings[4].location.path, PathBuf::from("cold.rs"));
        assert!(
            findings[0].risk_score.unwrap() > findings[4].risk_score.unwrap(),
            "hotspot bonus should boost score"
        );
    }

    #[test]
    fn sort_is_idempotent() {
        // Running prioritize twice must produce the same order.
        let mut a = vec![
            finding_at("a.rs", "x", Severity::Hint, Some(3.0), Some(1.0)),
            finding_at("b.rs", "y", Severity::Warning, None, None),
            finding_at("c.rs", "z", Severity::Error, None, None),
        ];
        prioritize_findings(&mut a);
        let first_order: Vec<String> = a.iter().map(|f| f.smell_name.clone()).collect();
        prioritize_findings(&mut a);
        let second_order: Vec<String> = a.iter().map(|f| f.smell_name.clone()).collect();
        assert_eq!(first_order, second_order);
    }
}
