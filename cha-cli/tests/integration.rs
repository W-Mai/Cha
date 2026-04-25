use std::process::Command;

fn cha_binary() -> String {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // up to workspace root
    path.push("target/release/cha");
    path.to_string_lossy().to_string()
}

fn fixture(name: &str) -> String {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    path.to_string_lossy().to_string()
}

fn run_analyze(file: &str, format: &str) -> (i32, String) {
    let output = Command::new(cha_binary())
        .args(["analyze", file, "--format", format])
        .output()
        .expect("failed to run cha");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (code, stdout)
}

#[test]
fn smelly_file_detects_naming_convention() {
    let (_, out) = run_analyze(&fixture("smelly.ts"), "terminal");
    assert!(
        out.contains("[naming_convention]"),
        "expected naming_convention finding"
    );
    assert!(out.contains("badClass"), "expected badClass in output");
}

#[test]
fn smelly_file_detects_long_parameter_list() {
    let (_, out) = run_analyze(&fixture("smelly.ts"), "terminal");
    assert!(
        out.contains("[long_parameter_list]"),
        "expected long_parameter_list finding"
    );
}

#[test]
fn smelly_file_detects_high_complexity() {
    let (_, out) = run_analyze(&fixture("smelly.ts"), "terminal");
    assert!(
        out.contains("[high_complexity]"),
        "expected high_complexity finding"
    );
}

#[test]
fn clean_file_has_no_warnings() {
    let (code, out) = run_analyze(&fixture("clean.ts"), "terminal");
    // Should not contain warning or error icons
    assert!(!out.contains("⚠"), "expected no warnings");
    assert!(!out.contains("✖"), "expected no errors");
    assert_eq!(code, 0);
}

#[test]
fn json_output_is_valid() {
    let (_, out) = run_analyze(&fixture("smelly.ts"), "json");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON output");
    assert!(parsed.is_object(), "JSON output should be an object");
    let findings = parsed.get("findings").expect("missing findings key");
    assert!(findings.is_array());
    let arr = findings.as_array().unwrap();
    assert!(!arr.is_empty(), "expected at least one finding");
    // Each finding should have required fields
    let first = &arr[0];
    assert!(first.get("smell_name").is_some());
    assert!(first.get("severity").is_some());
    assert!(first.get("message").is_some());
    // Health scores should be present
    let scores = parsed
        .get("health_scores")
        .expect("missing health_scores key");
    assert!(scores.is_array());
}

#[test]
fn sarif_output_is_valid() {
    let (_, out) = run_analyze(&fixture("smelly.ts"), "sarif");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("invalid SARIF output");
    assert_eq!(parsed["version"], "2.1.0");
    assert!(parsed["runs"].is_array());
}

#[test]
fn fail_on_warning_exits_nonzero() {
    let output = Command::new(cha_binary())
        .args(["analyze", &fixture("smelly.ts"), "--fail-on", "warning"])
        .output()
        .expect("failed to run cha");
    assert_ne!(
        output.status.code().unwrap_or(0),
        0,
        "expected nonzero exit"
    );
}

#[test]
fn fail_on_error_exits_zero_for_warnings_only() {
    let output = Command::new(cha_binary())
        .args(["analyze", &fixture("clean.ts"), "--fail-on", "error"])
        .output()
        .expect("failed to run cha");
    assert_eq!(output.status.code().unwrap_or(-1), 0);
}

#[test]
fn plugin_filter_limits_output() {
    // Only run the naming plugin — should not produce complexity findings
    let output = Command::new(cha_binary())
        .args([
            "analyze",
            &fixture("smelly.ts"),
            "--plugin",
            "naming",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run cha");
    let out = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    let findings = parsed["findings"].as_array().expect("missing findings");
    // All findings must come from the naming plugin
    for f in findings {
        let name = f["smell_name"].as_str().unwrap_or("");
        assert!(
            name.starts_with("naming"),
            "unexpected finding from non-naming plugin: {name}"
        );
    }
}

#[test]
fn plugin_filter_unknown_plugin_produces_no_findings() {
    let output = Command::new(cha_binary())
        .args([
            "analyze",
            &fixture("smelly.ts"),
            "--plugin",
            "nonexistent_plugin_xyz",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run cha");
    let out = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    let findings = parsed["findings"].as_array().expect("missing findings");
    assert!(
        findings.is_empty(),
        "expected no findings for unknown plugin"
    );
}

#[test]
fn focus_filter_keeps_only_matching_categories() {
    // The smelly.ts fixture produces findings across multiple categories;
    // --focus bloaters should leave only bloaters.
    let output = Command::new(cha_binary())
        .args([
            "analyze",
            &fixture("smelly.ts"),
            "--focus",
            "bloaters",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run cha");
    let out = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    let findings = parsed["findings"].as_array().expect("missing findings");
    for f in findings {
        assert_eq!(
            f["category"].as_str().unwrap_or(""),
            "bloaters",
            "non-bloater leaked through --focus bloaters: {}",
            f["smell_name"].as_str().unwrap_or("?")
        );
    }
}

#[test]
fn focus_filter_accepts_multiple_categories() {
    let output = Command::new(cha_binary())
        .args([
            "analyze",
            &fixture("smelly.ts"),
            "--focus",
            "bloaters,couplers",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run cha");
    let out = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    let findings = parsed["findings"].as_array().expect("missing findings");
    for f in findings {
        let cat = f["category"].as_str().unwrap_or("");
        assert!(
            cat == "bloaters" || cat == "couplers",
            "category `{cat}` passed --focus bloaters,couplers but shouldn't have"
        );
    }
}

#[test]
fn focus_filter_unknown_category_warns_but_does_not_crash() {
    let output = Command::new(cha_binary())
        .args([
            "analyze",
            &fixture("smelly.ts"),
            "--focus",
            "not_a_real_category",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run cha");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown"),
        "expected a warning for unknown category, got stderr: {stderr}"
    );
    let out = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("invalid JSON");
    let findings = parsed["findings"].as_array().expect("missing findings");
    assert!(
        findings.is_empty(),
        "unknown category should filter everything out"
    );
}

#[test]
fn llm_output_contains_findings_section() {
    let (_, out) = run_analyze(&fixture("smelly.ts"), "llm");
    assert!(
        out.contains("finding") || out.contains("smell") || out.contains("issue"),
        "LLM output missing expected content"
    );
}

#[test]
fn clean_file_exits_zero_with_fail_on_warning() {
    let output = Command::new(cha_binary())
        .args(["analyze", &fixture("clean.ts"), "--fail-on", "warning"])
        .output()
        .expect("failed to run cha");
    assert_eq!(output.status.code().unwrap_or(-1), 0);
}

// -- deps --direction tests --

fn run_deps(args: &[&str]) -> String {
    let output = Command::new(cha_binary())
        .arg("deps")
        .args(args)
        .output()
        .expect("failed to run cha deps");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn deps_direction_out_shows_only_outgoing() {
    let dir = fixture_dir("c_oop");
    let out = run_deps(&[
        &dir,
        "--type",
        "imports",
        "--filter",
        "widget.c",
        "--exact",
        "--direction",
        "out",
    ]);
    // widget.c imports widget.h → should appear
    assert!(
        out.contains("widget.h"),
        "direction=out should show widget.c → widget.h"
    );
}

#[test]
fn deps_direction_in_shows_only_incoming() {
    let dir = fixture_dir("c_oop");
    let out = run_deps(&[
        &dir,
        "--type",
        "imports",
        "--filter",
        "widget.h",
        "--exact",
        "--direction",
        "in",
    ]);
    // widget.c imports widget.h → widget.c should appear as source
    assert!(
        out.contains("widget.c"),
        "direction=in should show widget.c → widget.h"
    );
}

// -- deps --format plantuml tests --

#[test]
fn deps_format_plantuml_has_startuml() {
    let dir = fixture_dir("c_oop");
    let out = run_deps(&[&dir, "--type", "imports", "--format", "plantuml"]);
    assert!(
        out.starts_with("@startuml"),
        "plantuml output should start with @startuml"
    );
    assert!(
        out.contains("@enduml"),
        "plantuml output should end with @enduml"
    );
}

// -- C OOP filter test --

#[test]
fn c_oop_filter_suppresses_lazy_class_for_struct_with_methods() {
    let dir = fixture_dir("c_oop");
    let (_, out) = run_analyze(&dir, "json");
    // Widget has cross-file methods (widget_init, widget_draw) so should NOT be flagged
    assert!(
        !out.contains("\"lazy_class\""),
        "Widget should not be flagged as lazy_class because it has cross-file methods"
    );
}

fn fixture_dir(name: &str) -> String {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    path.to_string_lossy().to_string()
}

// -- S1: Language-adaptive thresholds --

#[test]
fn c_profile_no_long_method_under_100_lines() {
    // medium.c has a ~78-line function. C profile threshold is 100, so no long_method.
    let dir = fixture_dir("c_profile");
    let (_, out) = run_analyze(&dir, "json");
    assert!(
        !out.contains("\"long_method\""),
        "C file with <100 line function should not trigger long_method with C profile"
    );
}
