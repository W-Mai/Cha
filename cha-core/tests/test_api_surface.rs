mod common;

use cha_core::plugins::*;
use common::*;

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
