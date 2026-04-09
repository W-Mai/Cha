use std::io::{Read, Write};
use std::process::{Command, Stdio};

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

    // Commands that take extra arguments are handled separately
    if cmd == "bump" {
        let level = args.get(1).map(|s| s.as_str()).unwrap_or("");
        return cmd_bump(level);
    }
    if cmd == "publish" {
        let dry_run = args.iter().any(|a| a == "--dry-run");
        return cmd_publish(dry_run);
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
    ];
    if let Some((_, f)) = commands.iter().find(|(name, _)| *name == cmd) {
        f()
    } else {
        let names: Vec<&str> = commands.iter().map(|(n, _)| *n).collect();
        eprintln!(
            "usage: cargo xtask <{}|bump <major|minor|patch>|publish [--dry-run]>",
            names.join("|")
        );
        std::process::exit(1);
    }
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
        }]"#,
        );
    std::fs::write(&lib_rs, patched)?;
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
fn cmd_publish(dry_run: bool) -> Result {
    let root = project_root();

    // Check working tree is clean
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&root)
        .output()?;
    if !status.stdout.is_empty() {
        return Err("working tree is not clean — commit or stash changes first".into());
    }

    // (crate_name, working_dir relative to root)
    // sdk crates live in their own workspace
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
        let work_dir = format!("{root}/{dir}");
        let mut publish_args = vec!["publish", "-p", name, "--no-verify"];
        if dry_run {
            publish_args.push("--dry-run");
        }
        let status = Command::new("cargo")
            .args(&publish_args)
            .current_dir(&work_dir)
            .status()
            .map_err(|e| format!("failed to run cargo publish: {e}"))?;
        if !status.success() {
            return Err(format!("failed to publish {name}").into());
        }
        // Wait for crates.io index to update between publishes
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
/// Updates workspace Cargo.toml and cha-plugin-sdk/Cargo.toml + macros/Cargo.toml.
fn cmd_bump(level: &str) -> Result {
    if !matches!(level, "major" | "minor" | "patch") {
        return Err("usage: cargo xtask bump <major|minor|patch>".into());
    }
    let root = project_root();

    // Read current version from workspace Cargo.toml
    let ws_toml_path = format!("{root}/Cargo.toml");
    let ws_content = std::fs::read_to_string(&ws_toml_path)?;
    let current = ws_content
        .lines()
        .find(|l| l.trim().starts_with("version =") && !l.contains("workspace"))
        .and_then(|l| l.split('"').nth(1))
        .ok_or("could not find version in workspace Cargo.toml")?
        .to_string();

    let next = bump_version(&current, level)?;
    println!("  → bumping {current} → {next}");

    // Files to update: workspace + sdk workspace
    let targets = [
        format!("{root}/Cargo.toml"),
        format!("{root}/cha-plugin-sdk/Cargo.toml"),
        format!("{root}/cha-plugin-sdk/macros/Cargo.toml"),
    ];
    for path in &targets {
        let content = std::fs::read_to_string(path)?;
        // Replace version = "x.y.z" (not workspace = true lines)
        let updated = content
            .lines()
            .map(|line| {
                if line.trim().starts_with("version =") && !line.contains("workspace") {
                    line.replace(&format!("\"{current}\""), &format!("\"{next}\""))
                } else {
                    // Also update internal version references like version = "0.1.0" in deps
                    line.replace(
                        &format!("version = \"{current}\""),
                        &format!("version = \"{next}\""),
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(path, updated)?;
        println!("  → updated {path}");
    }

    println!("  ✅ version bumped to {next}");
    println!("  → run: git add -p && git commit -m \"🔖: bump version to {next}\"");
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
