use std::path::{Path, PathBuf};

const CARGO_TOML_TEMPLATE: &str = r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
cha-plugin-sdk = { git = "https://github.com/W-Mai/Cha" }
wit-bindgen = "0.55"
"#;

const LIB_RS_TEMPLATE: &str = r#"cha_plugin_sdk::plugin!(MyPlugin);

struct MyPlugin;

impl PluginImpl for MyPlugin {
    fn name() -> String {
        "{name}".into()
    }

    fn analyze(input: AnalysisInput) -> Vec<Finding> {
        let _ = input;
        vec![]
    }
}
"#;

pub fn cmd_new(name: &str) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let dir = if is_empty_dir(&cwd) {
        cwd.clone()
    } else {
        cwd.join(name)
    };

    if dir != cwd {
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| {
            eprintln!("error: failed to create directory {}: {e}", dir.display());
            std::process::exit(1);
        });
    }

    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).expect("failed to create src/");

    write_file(
        &dir.join("Cargo.toml"),
        &CARGO_TOML_TEMPLATE.replace("{name}", name),
    );
    write_file(
        &src_dir.join("lib.rs"),
        &LIB_RS_TEMPLATE.replace("{name}", name),
    );

    println!("Created plugin `{name}` in {}", dir.display());
    println!();
    println!("Next steps:");
    println!("  cd {}", dir.display());
    println!("  cargo build --target wasm32-wasip1 --release");
    println!("  cha plugin install target/wasm32-wasip1/release/{name}.wasm");
}

pub fn cmd_build() {
    let status = std::process::Command::new("cargo")
        .args(["build", "--target", "wasm32-wasip1", "--release"])
        .status();

    match status {
        Ok(s) if s.success() => {
            let wasm_name = read_package_name().unwrap_or_else(|| "plugin".into());
            let src = format!("target/wasm32-wasip1/release/{wasm_name}.wasm");
            let out = format!("{wasm_name}.wasm");
            match make_component(&src, &out) {
                Ok(()) => println!("Component ready: {out}\n  cha plugin install {out}"),
                Err(e) => eprintln!(
                    "warning: component conversion failed: {e}\n  Run wasm-tools manually."
                ),
            }
        }
        Ok(_) => std::process::exit(1),
        Err(e) => {
            eprintln!("error: failed to run cargo: {e}");
            std::process::exit(1);
        }
    }
}

fn make_component(src: &str, out: &str) -> anyhow::Result<()> {
    use wit_component::ComponentEncoder;
    let wasm = std::fs::read(src)?;
    let adapter = wasi_preview1_component_adapter_provider::WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER;
    let component = ComponentEncoder::default()
        .module(&wasm)?
        .adapter("wasi_snapshot_preview1", adapter)?
        .encode()?;
    std::fs::write(out, component)?;
    Ok(())
}

pub fn cmd_list() {
    let cwd = std::env::current_dir().unwrap_or_default();
    let local = cwd.join(".cha/plugins");
    let global = home_plugins_dir();

    let mut found = false;
    for (label, dir) in [
        ("local (.cha/plugins)", &local),
        ("global (~/.cha/plugins)", &global),
    ] {
        let plugins = list_wasm_files(dir);
        if !plugins.is_empty() {
            println!("{label}:");
            for p in plugins {
                print!("  {}", p.file_name().unwrap_or_default().to_string_lossy());
                if let Ok(wp) = cha_core::wasm::WasmPlugin::load(&p) {
                    print!("  v{}  {}", wp.version(), wp.description());
                    let authors = wp.authors();
                    if !authors.is_empty() {
                        print!("  ({})", authors.join(", "));
                    }
                }
                println!();
            }
            found = true;
        }
    }
    if !found {
        println!("No plugins installed.");
    }
}

pub fn cmd_install(path: &str) {
    let src = PathBuf::from(path);
    if !src.exists() {
        eprintln!("error: file not found: {path}");
        std::process::exit(1);
    }
    let dest_dir = std::env::current_dir()
        .unwrap_or_default()
        .join(".cha/plugins");
    std::fs::create_dir_all(&dest_dir).expect("failed to create .cha/plugins/");
    let dest = dest_dir.join(src.file_name().unwrap());
    std::fs::copy(&src, &dest).expect("failed to copy plugin");
    println!("Installed {} to {}", src.display(), dest.display());
}

pub fn cmd_remove(name: &str) {
    let dir = std::env::current_dir()
        .unwrap_or_default()
        .join(".cha/plugins");
    let target = find_plugin(&dir, name);
    match target {
        Some(p) => {
            std::fs::remove_file(&p).expect("failed to remove plugin");
            println!("Removed {}", p.display());
        }
        None => {
            eprintln!("error: plugin `{name}` not found in .cha/plugins/");
            std::process::exit(1);
        }
    }
}

fn is_empty_dir(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut d| d.next().is_none())
        .unwrap_or(false)
}

fn write_file(path: &Path, content: &str) {
    std::fs::write(path, content)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
}

fn list_wasm_files(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return vec![];
    }
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "wasm"))
        .collect()
}

fn find_plugin(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    let with_ext = dir.join(format!("{name}.wasm"));
    if with_ext.exists() {
        return Some(with_ext);
    }
    None
}

fn home_plugins_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".cha/plugins")
}

fn read_package_name() -> Option<String> {
    let content = std::fs::read_to_string("Cargo.toml").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("name") {
            let val = rest
                .trim_start_matches([' ', '=', '"'])
                .trim_end_matches('"');
            if !val.is_empty() {
                return Some(val.replace('-', "_"));
            }
        }
    }
    None
}
