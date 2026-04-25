//! Tests exercise `violation_to_findings` — the layer inference itself is
//! tested in `cha-core::graph`, so we just need to verify the adapter from
//! `LayerViolation` to `Finding`.

use super::*;
use cha_core::graph::LayerViolation;

#[test]
fn violation_to_single_finding_per_evidence_edge() {
    let v = LayerViolation {
        from_module: "domain".into(),
        to_module: "infra".into(),
        from_level: 0,
        to_level: 2,
        gap: 0.8,
        evidence: vec![
            ("src/domain/user.rs".into(), "src/infra/db.rs".into()),
            ("src/domain/order.rs".into(), "src/infra/db.rs".into()),
        ],
    };
    let cwd = PathBuf::from("/project");
    let findings = violation_to_findings(&v, &cwd);
    assert_eq!(findings.len(), 2);
    assert!(
        findings
            .iter()
            .all(|f| f.smell_name == "cross_layer_import")
    );
    assert!(findings[0].message.contains("domain"));
    assert!(findings[0].message.contains("infra"));
    assert!(findings[0].message.contains("0.80"));
    assert_eq!(
        findings[0].location.path,
        PathBuf::from("/project/src/domain/user.rs"),
        "finding anchors to the offending source file, not the target"
    );
}

#[test]
fn finding_carries_gap_as_metric() {
    let v = LayerViolation {
        from_module: "m1".into(),
        to_module: "m2".into(),
        from_level: 0,
        to_level: 1,
        gap: 0.5,
        evidence: vec![("a".into(), "b".into())],
    };
    let findings = violation_to_findings(&v, &PathBuf::from("/p"));
    assert_eq!(findings[0].actual_value, Some(0.5));
    assert_eq!(findings[0].threshold, Some(MIN_GAP));
    assert_eq!(findings[0].severity, Severity::Warning);
}
