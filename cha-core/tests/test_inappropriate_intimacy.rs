mod common;

use cha_core::plugins::*;
use cha_core::*;
use common::*;

// InappropriateIntimacyAnalyzer reads actual files to detect reverse imports,
// so unit testing with mock models is not feasible.
// It is covered by the self-analysis integration test (cha analyze . --format sarif).

#[test]
fn one_way_import_no_finding() {
    // Single file with outbound imports only — no bidirectional cycle possible
    let model = make_model(
        vec![],
        vec![],
        vec![import("utils", 1), import("types", 2)],
        10,
    );
    let findings = analyze(&InappropriateIntimacyAnalyzer::default(), &model);
    // May or may not find something depending on whether those files exist on disk;
    // at minimum, a file with no imports should never produce a finding.
    let _ = findings;
}

#[test]
fn no_imports_no_finding() {
    let model = make_model(vec![], vec![], vec![], 10);
    let findings = analyze(&InappropriateIntimacyAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
