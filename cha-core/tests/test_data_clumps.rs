mod common;

use cha_core::plugins::*;
use common::*;

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
