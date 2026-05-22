mod common;

use cha_core::Plugin;
use cha_core::plugins::*;
use common::*;

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

#[test]
fn message_chain_points_at_chain_expression() {
    // Real parse path: ctx.tree drives the precise location query.
    use cha_core::SourceFile;
    use std::path::PathBuf;
    let content = "\
fn deep() {
    let x = 1;
    let y = a.b.c.d.e;
}
";
    let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&lang).unwrap();
    let tree = parser.parse(content, None).unwrap();

    let mut f = func("deep", 4, 1, false);
    f.chain_depth = 4;
    let model = make_model(vec![f], vec![], vec![], 4);
    let file = SourceFile::new(PathBuf::from("test.rs"), content.into());

    let ctx = cha_core::AnalysisContext {
        file: &file,
        model: &model,
        tree: Some(&tree),
        ts_language: Some(&lang),
        project: None,
    };
    let findings = MessageChainAnalyzer::default().analyze(&ctx);
    assert_eq!(findings.len(), 1);
    // First field_expression in the function body is on line 3.
    assert_eq!(findings[0].location.start_line, 3);
}

#[test]
fn message_chain_falls_back_when_no_chain_text() {
    use cha_core::{AnalysisContext, SourceFile};
    use std::path::PathBuf;
    let mut f = func("synthetic", 10, 1, false);
    f.chain_depth = 5;
    f.name_col = 3;
    f.name_end_col = 12;
    let model = make_model(vec![f], vec![], vec![], 10);
    let file = SourceFile::new(PathBuf::from("test.rs"), "".into());
    let ctx = AnalysisContext {
        file: &file,
        model: &model,
        tree: None,
        ts_language: None,
        project: None,
    };
    let findings = MessageChainAnalyzer::default().analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].location.start_col, 3);
    assert_eq!(findings[0].location.end_col, 12);
}
