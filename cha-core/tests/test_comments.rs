mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

fn func_with_comments(name: &str, lines: usize, comment_lines: usize) -> FunctionInfo {
    let mut f = func(name, lines, 1, false);
    f.comment_lines = comment_lines;
    f
}

#[test]
fn comments_high_ratio_detected() {
    // 10 comment lines out of 20 total = 50% > 30% threshold
    let model = make_model(
        vec![func_with_comments("process", 20, 10)],
        vec![],
        vec![],
        20,
    );
    let findings = analyze(&CommentsAnalyzer::default(), &model);
    assert!(
        findings
            .iter()
            .any(|f| f.smell_name == "excessive_comments")
    );
}

#[test]
fn comments_low_ratio_ok() {
    let model = make_model(
        vec![func_with_comments("process", 20, 2)],
        vec![],
        vec![],
        20,
    );
    let findings = analyze(&CommentsAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn comments_zero_lines_ok() {
    let model = make_model(vec![func("tiny", 3, 1, false)], vec![], vec![], 3);
    let findings = analyze(&CommentsAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
