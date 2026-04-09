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
    match args.first().map(|s| s.as_str()) {
        Some("ci") => cmd_ci(),
        Some("build") => cmd_build(),
        Some("test") => cmd_test(),
        Some("lint") => cmd_lint(),
        Some("analyze") => cmd_analyze(),
        Some("lsp-test") => cmd_lsp_test(),
        Some("plugin-test") => cmd_plugin_test(),
        _ => {
            eprintln!("usage: cargo xtask <ci|build|test|lint|analyze|lsp-test|plugin-test>");
            std::process::exit(1);
        }
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
