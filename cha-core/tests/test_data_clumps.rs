mod common;

use cha_core::plugins::*;
use common::*;

#[test]
fn data_clumps_triggers() {
    let sig = vec![tref("String"), tref("i32"), tref("bool")];
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
    f1.parameter_types = vec![tref("i32"), tref("bool"), tref("String")];
    let mut f2 = func("b", 10, 1, false);
    f2.parameter_types = vec![tref("f64"), tref("Vec"), tref("Option")];
    let mut f3 = func("c", 10, 1, false);
    f3.parameter_types = vec![tref("u8"), tref("u16"), tref("u32")];
    let model = make_model(vec![f1, f2, f3], vec![], vec![], 30);
    let findings = analyze(&DataClumpsAnalyzer::default(), &model);
    assert!(findings.is_empty());
}

#[test]
fn data_clumps_below_min_occurrences() {
    let sig = vec![tref("i32"), tref("bool"), tref("String")];
    let mk = |name| {
        let mut f = func(name, 10, 1, false);
        f.parameter_types = sig.clone();
        f
    };
    let model = make_model(vec![mk("a"), mk("b")], vec![], vec![], 20);
    let findings = analyze(&DataClumpsAnalyzer::default(), &model);
    assert!(findings.is_empty());
}
