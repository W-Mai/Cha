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
    assert!(parsed.is_array(), "JSON output should be an array");
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty(), "expected at least one finding");
    // Each finding should have required fields
    let first = &arr[0];
    assert!(first.get("smell_name").is_some());
    assert!(first.get("severity").is_some());
    assert!(first.get("message").is_some());
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
    let findings: Vec<serde_json::Value> = serde_json::from_str(&out).expect("invalid JSON");
    // All findings must come from the naming plugin
    for f in &findings {
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
    let findings: Vec<serde_json::Value> = serde_json::from_str(&out).expect("invalid JSON");
    assert!(
        findings.is_empty(),
        "expected no findings for unknown plugin"
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
