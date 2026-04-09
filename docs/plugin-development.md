# Plugin Development Guide

This guide covers everything you need to write, test, and distribute a Cha WASM analyzer plugin.

## Prerequisites

- Rust toolchain with `wasm32-wasip1` target:
  ```bash
  rustup target add wasm32-wasip1
  ```
- Cha CLI installed and on `$PATH`

## Quick Start

```bash
mkdir my-plugin && cd my-plugin
cha plugin new my-plugin   # scaffold in current dir (empty) or creates subdir
cha plugin build           # compile + convert to WASM Component
cha plugin install my_plugin.wasm
cha analyze src/
```

## Scaffolding

`cha plugin new <name>` generates:

```
my-plugin/
  Cargo.toml   # cdylib + cha-plugin-sdk + wit-bindgen deps
  src/
    lib.rs     # plugin! macro + Guest impl stub
```

If the current directory is empty, files are generated in-place. Otherwise a `<name>/` subdirectory is created.

## Plugin Structure

```rust
cha_plugin_sdk::plugin!(MyPlugin);

struct MyPlugin;

impl PluginImpl for MyPlugin {
    fn name() -> String { "my-plugin".into() }
    fn analyze(input: AnalysisInput) -> Vec<Finding> { vec![] }
}
```

`plugin!` expands to `wit_bindgen::generate!` with the embedded WIT interface, plus `export!`. No local `.wit` file needed.

`version()`, `description()`, and `authors()` are automatically filled from the plugin's `Cargo.toml` — no need to implement them manually.

### Available Types

After `plugin!(MyPlugin)`, these types are in scope and `PluginImpl` is the trait to implement:

| Type | Description |
|------|-------------|
| `AnalysisInput` | Full file context passed to `analyze()` |
| `Finding` | A single detected issue |
| `FunctionInfo` | Per-function data (name, lines, complexity, …) |
| `ClassInfo` | Per-class data (name, methods, fields, …) |
| `ImportInfo` | Import source + line |
| `Location` | File path + line range |
| `Severity` | `Hint` / `Warning` / `Error` |
| `SmellCategory` | `Bloaters` / `Couplers` / `Dispensables` / … |
| `OptionValue` | `Str` / `Int` / `Float` / `Boolean` / `ListStr` |

### AnalysisInput Fields

```rust
pub struct AnalysisInput {
    pub path: String,          // file path
    pub content: String,       // raw source text
    pub language: String,      // "typescript" | "rust"
    pub total_lines: u32,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<ImportInfo>,
    pub options: Vec<(String, OptionValue)>,  // from .cha.toml
}
```

### FunctionInfo Fields

```rust
pub struct FunctionInfo {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub line_count: u32,
    pub complexity: u32,
    pub param_count: u32,
    pub param_types: Vec<String>,
    pub is_exported: bool,
    pub comment_lines: u32,
    pub referenced_fields: Vec<String>,
    pub null_check_fields: Vec<String>,
    pub switch_dispatch_target: Option<String>,
    pub optional_param_count: u32,
    pub body_hash: u64,
}
```

### ClassInfo Fields

```rust
pub struct ClassInfo {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub line_count: u32,
    pub method_count: u32,
    pub field_count: u32,
    pub field_names: Vec<String>,
    pub is_exported: bool,
    pub has_behavior: bool,
    pub is_interface: bool,
    pub parent_name: Option<String>,
    pub override_count: u32,
    pub self_call_count: u32,
    pub has_listener_field: bool,
    pub has_notify_method: bool,
}
```

## Reading Options

Options come from `.cha.toml`:

```toml
[plugins.my-plugin]
threshold = 10
label = "custom"
tags = ["a", "b"]
```

Use the SDK helper macros:

```rust
use cha_plugin_sdk::{option_int, option_str, option_list_str};

let threshold = option_int!(&input.options, "threshold").unwrap_or(5);
let label     = option_str!(&input.options, "label").unwrap_or("default");
let tags      = option_list_str!(&input.options, "tags").unwrap_or(&[]);
```

Available macros: `option_str!`, `option_int!`, `option_float!`, `option_bool!`, `option_list_str!`, `str_options!`.

## Building

```bash
cha plugin build
```

This runs `cargo build --target wasm32-wasip1 --release` and automatically converts the output to a WASM Component using the embedded WASI adapter. The result is `<name>.wasm` in the current directory.

## Installing

```bash
cha plugin install my_plugin.wasm        # project-local: .cha/plugins/
cp my_plugin.wasm ~/.cha/plugins/        # global
```

Cha loads plugins from both locations on every `analyze` run.

## Listing & Removing

```bash
cha plugin list
cha plugin remove my_plugin
```

## Configuration

Plugins are enabled by default once installed. Disable or configure in `.cha.toml`:

```toml
[plugins.my-plugin]
enabled = false

[plugins.my-plugin]
threshold = 20
```

The section name must match the string returned by `name()`.

## Testing

Add to `Cargo.toml`:

```toml
[dev-dependencies]
cha-plugin-sdk = { git = "https://github.com/W-Mai/Cha", features = ["test-utils"] }
```

Write tests:

```rust
#[cfg(test)]
mod tests {
    use cha_plugin_sdk::test_utils::WasmPluginTest;

    #[test]
    fn detects_issue() {
        WasmPluginTest::new()
            .source("typescript", "function todo_fix() {}")
            .assert_finding("my_smell_name");
    }

    #[test]
    fn no_false_positive() {
        WasmPluginTest::new()
            .source("typescript", "function processData() {}")
            .assert_no_finding();
    }

    #[test]
    fn respects_options() {
        WasmPluginTest::new()
            .source("typescript", r#"fetch("https://example.com");"#)
            .option("DOMAIN", "example.com")
            .assert_finding("hardcoded_string");
    }
}
```

Run with:

```bash
cha plugin build   # build wasm first (auto-triggered if missing)
cargo test
```

`WasmPluginTest` automatically calls `cha plugin build` if the `.wasm` file is not found.

### Assertion API

| Method | Description |
|--------|-------------|
| `.assert_any_finding()` | At least one finding |
| `.assert_no_finding()` | No findings at all |
| `.assert_finding("name")` | At least one finding with this smell name |
| `.assert_no_finding_named("name")` | No finding with this smell name |
| `.findings()` | Return `Vec<Finding>` for custom assertions |

## Example Plugins

| Plugin | Path | Detects |
|--------|------|---------|
| `example-wasm` | `examples/wasm-plugin-example` | Functions named todo/fixme/hack |
| `hardcoded-strings` | `examples/wasm-plugin-hardcoded` | Hardcoded string literals matching configured constants |

## WIT Interface

The full interface is in `wit/plugin.wit`. The `plugin!` macro embeds it at compile time — you never need to manage it manually.

```wit
world analyzer {
    export name: func() -> string;
    export version: func() -> string;       // auto from Cargo.toml
    export description: func() -> string;   // auto from Cargo.toml
    export authors: func() -> list<string>; // auto from Cargo.toml
    export analyze: func(input: analysis-input) -> list<finding>;
}
```
