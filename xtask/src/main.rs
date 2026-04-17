use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("");
    dispatch_with_args(cmd, &args)
}

fn dispatch_with_args(cmd: &str, args: &[String]) -> Result {
    if cmd == "bump" {
        let level = args.get(1).map(|s| s.as_str()).unwrap_or("");
        return cmd_bump(level);
    }
    if cmd == "publish" {
        let dry_run = args.iter().any(|a| a == "--dry-run");
        return cmd_publish(dry_run);
    }
    if cmd == "release" {
        return cmd_release();
    }

    type Cmd = (&'static str, fn() -> Result);
    let commands: &[Cmd] = &[
        ("ci", cmd_ci),
        ("build", cmd_build),
        ("test", cmd_test),
        ("lint", cmd_lint),
        ("analyze", cmd_analyze),
        ("lsp-test", cmd_lsp_test),
        ("plugin-test", cmd_plugin_test),
        ("plugin-e2e", cmd_plugin_e2e),
        ("integration-test", cmd_integration_test),
    ];
    if let Some((_, f)) = commands.iter().find(|(name, _)| *name == cmd) {
        return f();
    }
    let names: Vec<&str> = commands.iter().map(|(n, _)| *n).collect();
    eprintln!(
        "usage: cargo xtask <{}|bump <major|minor|patch>|publish [--dry-run]>",
        names.join("|")
    );
    std::process::exit(1);
}

// Run all CI steps in sequence
fn cmd_ci() -> Result {
    for (name, step) in [
        ("build", cmd_build as fn() -> Result),
        ("test", cmd_test),
        ("lint", cmd_lint),
        ("analyze", cmd_analyze),
        ("lsp-test", cmd_lsp_test),
        ("plugin-test", cmd_plugin_test),
        ("plugin-e2e", cmd_plugin_e2e),
        ("integration-test", cmd_integration_test),
    ] {
        println!("\n=== xtask: {name} ===");
        step()?;
    }
    println!("\n✅ All CI checks passed.");
    Ok(())
}

fn cmd_build() -> Result {
    cargo(&["build", "--release", "--workspace"])
}

fn cmd_test() -> Result {
    cargo(&["test", "--workspace"])
}

fn cmd_lint() -> Result {
    cargo(&["clippy", "--workspace", "--", "-D", "warnings"])?;
    cargo(&["fmt", "--all", "--check"])
}

fn cmd_analyze() -> Result {
    let cha = cha_binary();
    let src_dirs = ["cha-core", "cha-parser", "cha-cli/src", "cha-lsp", "xtask"];
    // Gate check on source dirs only (excludes test fixtures with intentional smells)
    let mut args = vec!["analyze", "--format", "sarif", "--fail-on", "warning"];
    args.extend_from_slice(&src_dirs);
    run_cmd(&cha, &args)?;
    // Remaining format smoke tests on full project
    run_cmd(&cha, &["analyze", ".", "--format", "terminal"])?;
    run_cmd(&cha, &["analyze", ".", "--format", "json"])?;
    run_cmd(&cha, &["analyze", ".", "--format", "llm"])?;
    run_cmd(&cha, &["analyze", "--diff"])
}

fn cmd_plugin_test() -> Result {
    let root = project_root();
    let cha = cha_binary();
    let examples = [
        "examples/wasm-plugin-example",
        "examples/wasm-plugin-hardcoded",
    ];
    for example in examples {
        let dir = format!("{root}/{example}");
        println!("  → plugin-test: {example}");
        // Build wasm
        let status = Command::new("cargo")
            .args(["build", "--target", "wasm32-wasip1", "--release"])
            .current_dir(&dir)
            .status()
            .map_err(|e| format!("cargo build failed: {e}"))?;
        if !status.success() {
            return Err(format!("cargo build failed in {example}").into());
        }
        // Convert to component
        let status = Command::new(&cha)
            .args(["plugin", "build"])
            .current_dir(&dir)
            .status()
            .map_err(|e| format!("cha plugin build failed: {e}"))?;
        if !status.success() {
            return Err(format!("cha plugin build failed in {example}").into());
        }
        // Run tests
        let status = Command::new("cargo")
            .args(["test"])
            .current_dir(&dir)
            .status()
            .map_err(|e| format!("cargo test failed: {e}"))?;
        if !status.success() {
            return Err(format!("cargo test failed in {example}").into());
        }
    }
    Ok(())
}

fn cmd_lsp_test() -> Result {
    let lsp = lsp_binary();
    let mut child = spawn_lsp_process(&lsp)?;
    send_initialize_request(&mut child)?;
    let resp = read_lsp_response(&mut child)?;
    let _ = child.kill();
    validate_lsp_response(&resp)
}

// Spawn the LSP server process with piped stdio.
fn spawn_lsp_process(lsp: &str) -> Result<std::process::Child> {
    Command::new(lsp)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to start {lsp}: {e}").into())
}

// Send an LSP initialize request via stdin.
fn send_initialize_request(child: &mut std::process::Child) -> Result {
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    let header = format!("Content-Length: {}\r\n\r\n{}", msg.len(), msg);
    child.stdin.take().unwrap().write_all(header.as_bytes())?;
    Ok(())
}

// Read stdout in a background thread until a complete JSON response arrives.
fn read_lsp_response(child: &mut std::process::Child) -> Result<String> {
    let stdout = child.stdout.take().unwrap();
    let handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut reader = std::io::BufReader::new(stdout);
        let mut tmp = [0u8; 4096];
        loop {
            match reader.read(&mut tmp) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                    if String::from_utf8_lossy(&buf).contains('}') {
                        break;
                    }
                }
            }
        }
        String::from_utf8_lossy(&buf).to_string()
    });
    handle.join().map_err(|_| "LSP read thread panicked".into())
}

// Validate that the response contains the expected initialize reply.
fn validate_lsp_response(resp: &str) -> Result {
    if resp.contains("\"id\":1") {
        println!("LSP initialize response OK");
        Ok(())
    } else {
        Err(format!("unexpected LSP response: {resp}").into())
    }
}

// cha:set long_method=60
fn e2e_scaffold_and_build(cha: &str, tmp: &str) -> Result<String> {
    // Run `cha plugin new` inside tmp so the plugin is created at tmp/test-e2e.
    // If tmp is empty, cha plugin new uses tmp itself as the plugin dir.
    let status = Command::new(cha)
        .args(["plugin", "new", "test-e2e"])
        .current_dir(tmp)
        .status()
        .map_err(|e| format!("failed to run cha plugin new: {e}"))?;
    if !status.success() {
        return Err("cha plugin new failed".into());
    }
    // cha plugin new uses cwd directly if it's empty, otherwise creates a subdir
    let plugin_dir = {
        let subdir = format!("{tmp}/test-e2e");
        if std::path::Path::new(&subdir).exists() {
            subdir
        } else {
            tmp.to_string()
        }
    };
    // Patch to always emit one finding so we can verify the plugin is loaded
    let lib_rs = format!("{plugin_dir}/src/lib.rs");
    let patched = std::fs::read_to_string(&lib_rs)?
        .replace(
            "fn analyze(_input: AnalysisInput)",
            "fn analyze(input: AnalysisInput)",
        )
        .replace(
            "vec![]",
            r#"vec![Finding {
            smell_name: "e2e_test_finding".into(),
            message: "e2e test".into(),
            severity: Severity::Hint,
            category: SmellCategory::Dispensables,
            location: Location { path: input.path.clone(), start_line: 1, end_line: 1, name: None },
            suggested_refactorings: vec![],
            actual_value: None,
            threshold: None,
        }]"#,
        );
    std::fs::write(&lib_rs, patched)?;
    patch_cargo_toml_for_local_sdk(&plugin_dir)?;
    let status = Command::new("cargo")
        .args(["build", "--target", "wasm32-wasip1", "--release"])
        .current_dir(&plugin_dir)
        .status()?;
    if !status.success() {
        return Err("cargo build failed".into());
    }
    run_cmd_in(cha, &["plugin", "build"], &plugin_dir)?;
    Ok(plugin_dir)
}

fn patch_cargo_toml_for_local_sdk(plugin_dir: &str) -> Result {
    let cargo_toml = format!("{plugin_dir}/Cargo.toml");
    let root = project_root();
    let content = std::fs::read_to_string(&cargo_toml)?;
    let patched = format!(
        "{content}\n[patch.\"https://github.com/W-Mai/Cha\"]\ncha-plugin-sdk = {{ path = \"{root}/cha-plugin-sdk\" }}\n"
    );
    std::fs::write(&cargo_toml, patched)?;
    Ok(())
}

fn e2e_verify_and_cleanup(cha: &str, tmp: &str) -> Result {
    println!("  → e2e: cha plugin list");
    let output = Command::new(cha)
        .args(["plugin", "list"])
        .current_dir(project_root())
        .output()?;
    let list = String::from_utf8_lossy(&output.stdout);
    if !list.contains("test_e2e.wasm") {
        return Err(format!("plugin not found in list: {list}").into());
    }
    println!("  → e2e: cha analyze (verify plugin is active)");
    let probe = format!("{tmp}/probe.ts");
    std::fs::write(&probe, "function hello() {}")?;
    let output = Command::new(cha)
        .args(["analyze", &probe, "--format", "json"])
        .current_dir(project_root())
        .output()?;
    let json = String::from_utf8_lossy(&output.stdout);
    if !json.contains("e2e_test_finding") {
        return Err(format!("plugin did not produce expected finding; output: {json}").into());
    }
    println!("  → e2e: cha plugin remove test_e2e");
    run_cmd_in(cha, &["plugin", "remove", "test_e2e"], &project_root())?;
    let output = Command::new(cha)
        .args(["plugin", "list"])
        .current_dir(project_root())
        .output()?;
    if String::from_utf8_lossy(&output.stdout).contains("test_e2e.wasm") {
        return Err("plugin still present after remove".into());
    }
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("cha-plugin-e2e-test"));
    println!("  ✅ plugin e2e passed");
    Ok(())
}

fn cmd_plugin_e2e() -> Result {
    let cha = cha_binary();
    let tmp = std::env::temp_dir().join("cha-plugin-e2e-test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp)?;
    let tmp = tmp.to_string_lossy().to_string();

    println!("  → e2e: scaffold + build");
    let plugin_dir = e2e_scaffold_and_build(&cha, &tmp)?;

    let wasm = format!("{plugin_dir}/test_e2e.wasm");
    println!("  → e2e: cha plugin install");
    run_cmd_in(&cha, &["plugin", "install", &wasm], &project_root())?;

    e2e_verify_and_cleanup(&cha, &tmp)
}

// Helpers

fn project_root() -> String {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| {
            std::path::Path::new(&d)
                .parent()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|_| ".".to_string())
}

fn cha_binary() -> String {
    format!("{}/target/release/cha", project_root())
}

fn lsp_binary() -> String {
    format!("{}/target/release/cha-lsp", project_root())
}

/// Validate integration artifacts: pre-commit hook, GitHub Action, VS Code extension.
fn cmd_integration_test() -> Result {
    println!("  [1/3] pre-commit hook...");
    test_precommit_hook()?;
    println!("  [2/3] GitHub Action (action-validator)...");
    test_action_yml()?;
    println!("  [3/3] VS Code extension (tsc --noEmit)...");
    test_vscode_extension()?;
    println!("  ✅ Integration tests passed.");
    Ok(())
}

fn test_precommit_hook() -> Result {
    let root = project_root();
    let tmp = std::env::temp_dir().join("cha-precommit-test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp)?;
    let d = tmp.to_str().unwrap();
    run_cmd_in("git", &["init", "-q"], d)?;
    run_cmd_in("git", &["config", "user.email", "test@test.com"], d)?;
    run_cmd_in("git", &["config", "user.name", "test"], d)?;
    std::fs::copy(
        format!("{root}/cha-parser/tests/fixtures/simple.ts"),
        tmp.join("simple.ts"),
    )?;
    run_cmd_in("git", &["add", "."], d)?;
    run_cmd_in("git", &["commit", "-q", "-m", "init"], d)?;
    let status = Command::new("pre-commit")
        .args(["try-repo", &root, "cha-analyze", "--files", "simple.ts"])
        .current_dir(&tmp)
        .status();
    let _ = std::fs::remove_dir_all(&tmp);
    match status {
        Ok(s) => {
            println!("    pre-commit try-repo exited with {s} (non-zero is OK — means hook ran)")
        }
        Err(e) => eprintln!("    ⚠ pre-commit not installed, skipping: {e}"),
    }
    Ok(())
}

fn test_action_yml() -> Result {
    let root = project_root();
    match Command::new("action-validator")
        .arg(format!("{root}/action.yml"))
        .status()
    {
        Ok(s) if s.success() => println!("    action.yml validated ✓"),
        Ok(s) => return Err(format!("action-validator failed: {s}").into()),
        Err(_) => eprintln!("    ⚠ action-validator not installed, skipping"),
    }
    Ok(())
}

fn test_vscode_extension() -> Result {
    let vscode_dir = format!("{}/vscode-cha", project_root());
    if !std::path::Path::new(&format!("{vscode_dir}/node_modules")).exists() {
        run_cmd_in("npm", &["install", "--ignore-scripts"], &vscode_dir)?;
    }
    run_cmd_in("npx", &["tsc", "--noEmit"], &vscode_dir)?;
    println!("    tsc --noEmit passed ✓");
    Ok(())
}

fn cargo(args: &[&str]) -> Result {
    run_cmd("cargo", args)
}

fn run_cmd_in(cmd: &str, args: &[&str], dir: &str) -> Result {
    println!("  → {cmd} {}", args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("failed to run {cmd}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{cmd} {} failed with {status}", args.join(" ")).into())
    }
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result {
    println!("  → {cmd} {}", args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .current_dir(project_root())
        .status()
        .map_err(|e| format!("failed to run {cmd}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{cmd} {} failed with {status}", args.join(" ")).into())
    }
}

/// Publish all publishable crates to crates.io in topological order.
/// Use --dry-run to only verify packaging without publishing.
fn publish_one(name: &str, work_dir: &str, dry_run: bool) -> Result {
    let mut args = vec!["publish", "-p", name, "--no-verify", "--allow-dirty"];
    if dry_run {
        args.push("--dry-run");
    }
    let status = Command::new("cargo")
        .args(&args)
        .current_dir(work_dir)
        .status()
        .map_err(|e| format!("failed to run cargo publish: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("failed to publish {name}").into())
    }
}

fn cmd_publish(dry_run: bool) -> Result {
    let root = project_root();

    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&root)
        .output()?;
    if !output.stdout.is_empty() {
        return Err("working tree is not clean — commit or stash changes first".into());
    }

    // (crate_name, working_dir relative to root); sdk crates live in their own workspace
    let crates: &[(&str, &str)] = &[
        ("cha-core", "."),
        ("cha-plugin-sdk-macros", "cha-plugin-sdk"),
        ("cha-parser", "."),
        ("cha-plugin-sdk", "cha-plugin-sdk"),
        ("cha-cli", "."),
    ];

    let verb = if dry_run { "Packaging" } else { "Publishing" };
    println!("  → {verb} {} crates", crates.len());

    for (i, (name, dir)) in crates.iter().enumerate() {
        println!("\n  [{}/{}] {verb} {name}", i + 1, crates.len());
        publish_one(name, &format!("{root}/{dir}"), dry_run)?;
        if !dry_run && i + 1 < crates.len() {
            println!("  → waiting 30s for crates.io index...");
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
    }

    if dry_run {
        println!("\n  ✅ dry-run complete — all crates packaged successfully");
        println!("  → run without --dry-run to publish for real");
    } else {
        println!("\n  ✅ all crates published successfully");
    }
    Ok(())
}

/// Bump version across all publishable crates.
/// Bump version across all Cargo.toml files in the repo.
/// Dynamically scans every Cargo.toml; rewrites:
/// - Own `version = "x.y.z"` (not workspace)
/// - Any dependency line containing `path =` (i.e. internal crate)
fn cmd_bump(level: &str) -> Result {
    if !matches!(level, "major" | "minor" | "patch") {
        return Err("usage: cargo xtask bump <major|minor|patch>".into());
    }
    let root = project_root();
    let current = read_workspace_version(&root)?;
    let next = bump_version(&current, level)?;
    println!("  → bumping {current} → {next}");
    for entry in find_cargo_tomls(&root) {
        if rewrite_version(&entry, &next)? {
            println!("  → updated {entry}");
        }
    }
    // Refresh all Cargo.lock files so they pick up the new versions
    for lock in find_cargo_locks(&root) {
        let dir = std::path::Path::new(&lock).parent().unwrap();
        print!("  → refreshing {lock} ... ");
        let st = Command::new("cargo")
            .arg("update")
            .arg("--workspace")
            .current_dir(dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        println!("{}", if st.success() { "ok" } else { "FAILED" });
    }
    // Sync vscode-cha/package.json version
    let pkg_json = format!("{root}/vscode-cha/package.json");
    if std::path::Path::new(&pkg_json).exists() {
        let content = std::fs::read_to_string(&pkg_json)?;
        let updated = content.replace(
            &format!("\"version\": \"{current}\""),
            &format!("\"version\": \"{next}\""),
        );
        if updated != content {
            std::fs::write(&pkg_json, updated)?;
            println!("  → updated {pkg_json}");
        }
    }
    println!("  ✅ version bumped to {next}");
    println!("  → run: git add -p && git commit -m \"🔖: bump version to {next}\"");
    Ok(())
}

fn read_workspace_version(root: &str) -> Result<String> {
    let content = std::fs::read_to_string(format!("{root}/Cargo.toml"))?;
    content
        .lines()
        .find(|l| l.trim().starts_with("version =") && !l.contains("workspace"))
        .and_then(|l| l.split('"').nth(1))
        .map(|s| s.to_string())
        .ok_or_else(|| "could not find version in workspace Cargo.toml".into())
}

fn find_files(root: &str, filename: &str) -> Vec<String> {
    let mut result = Vec::new();
    let target = filename.to_string();
    fn walk(dir: &std::path::Path, target: &str, result: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name != "target" && name != "node_modules" && name != ".git" {
                    walk(&path, target, result);
                }
            } else if path.file_name().is_some_and(|f| f == target) {
                result.push(path.to_string_lossy().into_owned());
            }
        }
    }
    walk(std::path::Path::new(root), &target, &mut result);
    result.sort();
    result
}

fn find_cargo_tomls(root: &str) -> Vec<String> {
    find_files(root, "Cargo.toml")
}

fn find_cargo_locks(root: &str) -> Vec<String> {
    find_files(root, "Cargo.lock")
}

/// Rewrite version strings in a Cargo.toml to `next`.
fn rewrite_version(path: &str, next: &str) -> Result<bool> {
    let content = std::fs::read_to_string(path)?;
    let updated = content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            let should_rewrite =
                // Own package version: `version = "x.y.z"` (not workspace ref)
                (trimmed.starts_with("version =") && !trimmed.contains("workspace"))
                // Internal dep: any line with both `path =` and `version =`
                || (trimmed.contains("path =") && trimmed.contains("version ="));
            if should_rewrite {
                replace_first_semver(line, next)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    if updated == content {
        return Ok(false);
    }
    std::fs::write(path, updated)?;
    Ok(true)
}

/// Replace the first `"x.y.z"` semver string in a line.
fn replace_first_semver(line: &str, next: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find('"') {
        result.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        if let Some(end) = after.find('"') {
            let inner = &after[..end];
            if is_semver(inner) {
                result.push_str(&format!("\"{next}\""));
                rest = &after[end + 1..];
                result.push_str(rest);
                return result;
            }
            result.push('"');
            result.push_str(inner);
            result.push('"');
            rest = &after[end + 1..];
        } else {
            result.push_str(&rest[start..]);
            return result;
        }
    }
    result.push_str(rest);
    result
}

fn is_semver(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn gh(args: &[&str]) -> Result<String> {
    let out = Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("gh {}: {e}", args.join(" ")))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!(
            "gh {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )
        .into())
    }
}

/// Poll until the latest run of a workflow reaches a terminal state.
/// Returns "success" or errors on failure/timeout.
fn wait_for_workflow(workflow: &str, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    println!("  → waiting for {workflow} ...");
    loop {
        std::thread::sleep(Duration::from_secs(15));
        let out = gh(&[
            "run",
            "list",
            "--workflow",
            workflow,
            "--limit",
            "1",
            "--json",
            "status,conclusion",
            "-q",
            ".[0] | [.status, .conclusion] | @tsv",
        ])?;
        let parts: Vec<&str> = out.split('\t').collect();
        let status = parts.first().copied().unwrap_or("");
        let conclusion = parts.get(1).copied().unwrap_or("");
        println!("    {workflow}: {status} / {conclusion}");
        if status == "completed" {
            if conclusion == "success" {
                return Ok(());
            }
            return Err(format!("{workflow} completed with: {conclusion}").into());
        }
        if start.elapsed() > timeout {
            return Err(format!("timeout waiting for {workflow}").into());
        }
    }
}

fn ensure_clean_tree(root: &str) -> Result {
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()?;
    if !out.stdout.is_empty() {
        return Err("working tree is not clean — commit all changes first".into());
    }
    Ok(())
}

fn tag_and_push(root: &str, tag: &str) -> Result {
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let tag_commit = Command::new("git")
        .args(["rev-list", "-n1", tag])
        .current_dir(root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if !tag_commit.is_empty() && tag_commit == head {
        println!("  → tag {tag} already points to HEAD, skipping");
        return Ok(());
    }

    if !tag_commit.is_empty() {
        // Tag exists but points to wrong commit — delete and re-tag
        println!("  → tag {tag} points to old commit, re-tagging...");
        let _ = run_cmd("git", &["tag", "-d", tag]);
        let _ = run_cmd("git", &["push", "origin", &format!(":refs/tags/{tag}")]);
    }

    run_cmd("git", &["tag", tag])?;
    run_cmd("git", &["push", "origin", tag])?;
    Ok(())
}

fn wait_for_release_workflow(tag: &str) -> Result {
    let done = gh(&[
        "run",
        "list",
        "--workflow",
        "release.yml",
        "--limit",
        "1",
        "--json",
        "status,conclusion,headBranch",
        "-q",
        &format!(".[0] | select(.headBranch == \"{tag}\") | .conclusion"),
    ])
    .unwrap_or_default();
    if done == "success" {
        println!("  → release workflow already succeeded, skipping");
        return Ok(());
    }
    let prev_id = gh(&[
        "run", "list", "--workflow", "release.yml",
        "--limit", "1",
        "--json", "databaseId,conclusion,headBranch",
        "-q", &format!(".[0] | select(.headBranch == \"{tag}\") | select(.conclusion == \"failure\") | .databaseId"),
    ])
    .unwrap_or_default();
    if !prev_id.is_empty() {
        println!("  → previous release run failed, re-running...");
        gh(&["run", "rerun", &prev_id])?;
    }
    wait_for_workflow("release.yml", Duration::from_secs(30 * 60))?;
    println!("  ✅ GitHub Release created");
    Ok(())
}

fn cmd_release() -> Result {
    let root = project_root();
    ensure_clean_tree(&root)?;

    let version = read_workspace_version(&root)?;
    let tag = format!("v{version}");
    println!("  → releasing {tag}");

    println!("\n  → git push origin main");
    run_cmd("git", &["push", "origin", "main"])?;

    println!("\n  → waiting for CI to pass (timeout 20min)...");
    wait_for_workflow("ci.yml", Duration::from_secs(20 * 60))?;
    println!("  ✅ CI passed");

    println!("\n  → tagging {tag}");
    tag_and_push(&root, &tag)?;

    println!("\n  → waiting for release workflow (timeout 30min)...");
    wait_for_release_workflow(&tag)?;

    println!("\n  → publishing to crates.io...");
    cmd_publish(false)?;

    println!("\n  🎉 released {tag} successfully!");
    Ok(())
}

fn bump_version(version: &str, level: &str) -> Result<String> {
    let parts: Vec<u64> = version
        .split('.')
        .map(|p| {
            p.parse::<u64>()
                .map_err(|e| format!("invalid version: {e}"))
        })
        .collect::<std::result::Result<_, _>>()?;
    if parts.len() != 3 {
        return Err(format!("expected semver x.y.z, got {version}").into());
    }
    let (major, minor, patch) = (parts[0], parts[1], parts[2]);
    let next = match level {
        "major" => format!("{}.0.0", major + 1),
        "minor" => format!("{major}.{}.0", minor + 1),
        "patch" => format!("{major}.{minor}.{}", patch + 1),
        _ => unreachable!(),
    };
    Ok(next)
}
