mod common;

use cha_core::plugins::ErrorHandlingAnalyzer;
use cha_core::{AnalysisContext, Plugin, SourceFile};
use common::*;
use std::path::PathBuf;

fn file(content: &str) -> SourceFile {
    SourceFile::new(PathBuf::from("test.rs"), content.into())
}

#[test]
fn unwrap_abuse_per_site_not_per_function() {
    // Function contains 5 unwraps on separate lines, threshold 3 → 5 findings.
    let content = "\
fn bad() {
    a.unwrap();
    b.unwrap();
    c.unwrap();
    d.unwrap();
    e.unwrap();
}
";
    let f = func("bad", 7, 1, false);
    let model = make_model(vec![f], vec![], vec![], 7);
    let src = file(content);
    let ctx = AnalysisContext {
        file: &src,
        model: &model,
    };
    let findings = ErrorHandlingAnalyzer::default().analyze(&ctx);
    let unwrap_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.smell_name == "unwrap_abuse")
        .collect();
    assert_eq!(
        unwrap_findings.len(),
        5,
        "expected one finding per call site"
    );
}

#[test]
fn unwrap_abuse_location_points_at_call() {
    let content = "\
fn bad() {
    foo.unwrap();
    bar.unwrap();
    baz.unwrap();
    qux.unwrap();
}
";
    let f = func("bad", 6, 1, false);
    let model = make_model(vec![f], vec![], vec![], 6);
    let src = file(content);
    let ctx = AnalysisContext {
        file: &src,
        model: &model,
    };
    let findings = ErrorHandlingAnalyzer::default().analyze(&ctx);
    let unwraps: Vec<_> = findings
        .iter()
        .filter(|f| f.smell_name == "unwrap_abuse")
        .collect();
    assert!(unwraps.len() >= 4);
    // Each finding's location should point at the actual `.unwrap()` substring,
    // not at line 1 (function header).
    for f in &unwraps {
        assert!(
            f.location.start_line >= 2,
            "expected per-call location, got line {}",
            f.location.start_line
        );
        assert!(
            f.location.end_col > f.location.start_col,
            "end_col should be past start_col"
        );
    }
}

#[test]
fn unwrap_abuse_skipped_when_under_threshold() {
    let content = "\
fn ok() {
    a.unwrap();
    b.unwrap();
}
";
    let f = func("ok", 4, 1, false);
    let model = make_model(vec![f], vec![], vec![], 4);
    let src = file(content);
    let ctx = AnalysisContext {
        file: &src,
        model: &model,
    };
    let findings = ErrorHandlingAnalyzer::default().analyze(&ctx);
    assert!(
        findings.iter().all(|f| f.smell_name != "unwrap_abuse"),
        "2 unwraps is under the default threshold of 3, should not flag"
    );
}

#[test]
fn unwrap_abuse_handles_multiple_on_same_line() {
    // Chained unwraps: 4 on one line, threshold 3 → 4 findings.
    let content = "\
fn chained() {
    a.unwrap().b.unwrap().c.unwrap().d.unwrap();
}
";
    let f = func("chained", 3, 1, false);
    let model = make_model(vec![f], vec![], vec![], 3);
    let src = file(content);
    let ctx = AnalysisContext {
        file: &src,
        model: &model,
    };
    let findings = ErrorHandlingAnalyzer::default().analyze(&ctx);
    let unwraps: Vec<_> = findings
        .iter()
        .filter(|f| f.smell_name == "unwrap_abuse")
        .collect();
    assert_eq!(
        unwraps.len(),
        4,
        "each chained unwrap should get its own finding"
    );
    // Each should have a distinct start_col on the same line.
    let mut cols: Vec<_> = unwraps.iter().map(|f| f.location.start_col).collect();
    cols.sort();
    cols.dedup();
    assert_eq!(cols.len(), 4, "4 distinct columns expected");
}
