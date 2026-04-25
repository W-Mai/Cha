mod common;

use cha_core::Plugin;
use cha_core::plugins::*;
use common::*;

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

#[test]
fn switch_statement_points_at_match_keyword() {
    use cha_core::{AnalysisContext, SourceFile};
    use std::path::PathBuf;
    let content = "\
fn foo() {
    let x = 1;
    match x {
        1 => {}
        _ => {}
    }
}
";
    let mut f = func("foo", 7, 1, false);
    f.switch_arms = 9;
    let model = make_model(vec![f], vec![], vec![], 7);
    let file = SourceFile::new(PathBuf::from("test.rs"), content.into());
    let ctx = AnalysisContext {
        file: &file,
        model: &model,
    };
    let findings = SwitchStatementAnalyzer::default().analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].location.start_line, 3);
    assert_eq!(findings[0].location.start_col, 4);
    assert_eq!(findings[0].location.end_col, 9);
}

#[test]
fn switch_statement_falls_back_when_keyword_not_found() {
    use cha_core::{AnalysisContext, SourceFile};
    use std::path::PathBuf;
    let mut f = func("synthetic", 20, 1, false);
    f.switch_arms = 9;
    f.name_col = 3;
    f.name_end_col = 12;
    let model = make_model(vec![f], vec![], vec![], 20);
    let file = SourceFile::new(PathBuf::from("test.rs"), "".into());
    let ctx = AnalysisContext {
        file: &file,
        model: &model,
    };
    let findings = SwitchStatementAnalyzer::default().analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].location.start_col, 3);
    assert_eq!(findings[0].location.end_col, 12);
}
