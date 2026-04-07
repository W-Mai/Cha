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
        _ => {
            eprintln!("usage: cargo xtask <ci|build|test|lint|analyze|lsp-test>");
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
    // Self-analyze using the just-built binary
    let cha = cha_binary();
    run_cmd(&cha, &["analyze", ".", "--format", "terminal"])?;
    run_cmd(&cha, &["analyze", ".", "--format", "json"])?;
    run_cmd(
        &cha,
        &["analyze", ".", "--format", "sarif", "--fail-on", "warning"],
    )?;
    run_cmd(&cha, &["analyze", ".", "--format", "llm"])?;
    run_cmd(&cha, &["analyze", "--diff"])
}

fn cmd_lsp_test() -> Result {
    let lsp = lsp_binary();
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    let header = format!("Content-Length: {}\r\n\r\n{}", msg.len(), msg);

    let mut child = Command::new(&lsp)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to start {}: {e}", lsp))?;

    child.stdin.take().unwrap().write_all(header.as_bytes())?;

    let stdout = child.stdout.take().unwrap();
    let handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut reader = std::io::BufReader::new(stdout);
        // Read until we get a complete JSON response
        let mut tmp = [0u8; 4096];
        loop {
            match reader.read(&mut tmp) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                    let s = String::from_utf8_lossy(&buf);
                    if s.contains('}') {
                        break;
                    }
                }
            }
        }
        String::from_utf8_lossy(&buf).to_string()
    });

    let resp = match handle.join() {
        Ok(r) => r,
        Err(_) => return Err("LSP read thread panicked".into()),
    };
    let _ = child.kill();

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
