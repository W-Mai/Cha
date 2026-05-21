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
    fn smells() -> Vec<String> { vec!["my_smell".into()] }
    fn analyze(input: AnalysisInput) -> Vec<Finding> { vec![] }
}
```

`plugin!` expands to `wit_bindgen::generate!` with the embedded WIT interface, plus `export!`. No local `.wit` file needed.

### Declaring smells

Every `Finding` carries a `smell_name`. Declaring the full set in `smells()` lets the host:
- List your plugin's smells in `cha plugin list`
- Let users disable specific smells via `disabled_smells = ["your_smell"]` in `.cha.toml`
- Pass the disabled-smells list back to your plugin so you can skip work early

Skip disabled smells efficiently:

```rust
use cha_plugin_sdk::is_smell_disabled;

fn analyze(input: AnalysisInput) -> Vec<Finding> {
    let mut out = Vec::new();
    if !is_smell_disabled!(&input.options, "my_smell") {
        // only compute my_smell if it isn't disabled
    }
    out
}
```

The host also post-filters findings whose `smell_name` is disabled, so forgetting to call `is_smell_disabled!` won't surface false positives — it just wastes work.

`version()`, `description()`, and `authors()` are automatically filled from the plugin's `Cargo.toml` — no need to implement them manually.

### Available Types

After `plugin!(MyPlugin)`, these types are in scope and `PluginImpl` is the trait to implement:

| Type | Description |
|------|-------------|
| `AnalysisInput` | Full file context passed to `analyze()` |
| `Finding` | A single detected issue |
| `FunctionInfo` | Per-function data (name, lines, complexity, params, …) |
| `ClassInfo` | Per-class data (name, methods, fields, …) |
| `ImportInfo` | Import source + line + is_module_decl |
| `CommentInfo` | Comment text + line |
| `ArmValue` | Switch/match arm value (`StrLit` / `IntLit` / `CharLit` / `Other`) |
| `FileRole` | `Source` / `Test` / `Doc` / `Config` / `Generated` |
| `Location` | File path + line/column range |
| `Severity` | `Hint` / `Warning` / `Error` |
| `SmellCategory` | `Bloaters` / `Couplers` / `Dispensables` / … |
| `OptionValue` | `Str` / `Int` / `Float` / `Boolean` / `ListStr` |
| `tree_query` | Module for AST queries (see below) |

### AnalysisInput Fields

```rust
pub struct AnalysisInput {
    pub path: String,             // file path
    pub content: String,          // raw source text
    pub language: String,         // "typescript" | "rust" | "python" | "go" | "c" | "cpp"
    pub total_lines: u32,
    pub role: FileRole,           // Source / Test / Doc / Config / Generated
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<ImportInfo>,
    pub comments: Vec<CommentInfo>,
    pub type_aliases: Vec<(String, String)>,
    pub options: Vec<(String, OptionValue)>,  // from .cha.toml
}
```

> **Note:** WASM plugins run in a sandboxed environment with no filesystem access.
> Use `input.content` to read the source text — do **not** use `std::fs::read_to_string(&input.path)`, it will silently return an empty string.

### File Role

The `role` field tells you what kind of file is being analyzed. Use it to apply different rules:

```rust
fn analyze(input: AnalysisInput) -> Vec<Finding> {
    if input.role == FileRole::Test {
        return vec![];  // skip detection for test files
    }
    // ...
}
```

### AST Query API (`tree_query`)

Plugins can execute tree-sitter queries against the current file's AST via host callbacks:

```rust
fn analyze(input: AnalysisInput) -> Vec<Finding> {
    // Find all unsafe blocks in the file
    let matches = tree_query::run_query("(unsafe_block) @blk");
    for m in &matches {
        for capture in m {
            // capture.node_kind, capture.text, capture.start_line, ...
        }
    }

    // Batch multiple queries in one call (reduces overhead)
    let results = tree_query::run_queries(&[
        "(if_statement) @if".into(),
        "(for_statement) @for".into(),
    ]);

    // Get node at a specific position
    if let Some(node) = tree_query::node_at(10, 4) {
        // node.node_kind, node.text, ...
    }

    // Get all named top-level nodes in a line range
    let nodes = tree_query::nodes_in_range(1, 50);

    vec![]
}
```

The query pattern syntax is [tree-sitter's S-expression query language](https://tree-sitter.github.io/tree-sitter/syntax-highlighting/queries). Queries are compiled and cached per-invocation on the host side.

Each `QueryMatch` contains:
- `capture_name` — the `@name` from the pattern
- `node_kind` — tree-sitter node type (e.g. `"function_definition"`)
- `text` — the matched source text
- `start_line`, `end_line` — **1-based** line numbers (matching `FunctionInfo.start_line`, `ClassInfo.start_line`, etc.)
- `start_col`, `end_col` — **0-based** byte columns

> **Line/column convention**: All line numbers in the SDK (functions, classes, comments, tree-query matches) are 1-based. Columns are 0-based byte offsets. Mixing the two is a common bug source — always read which axis you're on.

### Project Query API (`project_query`)

For cross-file analysis (callers, type origin, function bodies in other files), plugins call `project_query` host functions:

```rust
fn analyze(input: AnalysisInput) -> Vec<Finding> {
    // Is this name called from any file other than the current one?
    let unused = !project_query::is_called_externally(&fn_name, &input.path);

    // Which files reference this function?
    let callers = project_query::callers_of(&fn_name);

    // Find which function declaration contains a position
    // (1-based line, 0-based col — same as tree_query)
    if let Some(host_fn) = project_query::function_at(&input.path, line, col) {
        // host_fn.start_line, host_fn.end_line — both 1-based
    }

    // Type origin classification
    if project_query::is_third_party(&type_ref) {
        // External crate, not stdlib, not workspace sibling
    }

    // Path shape
    if project_query::is_test_path(&input.path) { /* ... */ }

    vec![]
}
```

`function_at` is especially useful for tree-query–driven detectors that need to know which declared function a queried position belongs to (e.g. distinguishing "early-return + later hook" between sibling components in the same file).

### FunctionInfo Fields

```rust
pub struct FunctionInfo {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub name_col: u32,
    pub name_end_col: u32,
    pub line_count: u32,
    pub complexity: u32,
    pub parameter_count: u32,
    pub parameter_types: Vec<TypeRef>,
    pub parameter_names: Vec<String>,
    pub chain_depth: u32,
    pub switch_arms: u32,
    pub switch_arm_values: Vec<ArmValue>,
    pub external_refs: Vec<String>,
    pub is_delegating: bool,
    pub is_exported: bool,
    pub comment_lines: u32,
    pub referenced_fields: Vec<String>,
    pub null_check_fields: Vec<String>,
    pub switch_dispatch_target: Option<String>,
    pub optional_param_count: u32,
    pub called_functions: Vec<String>,
    pub cognitive_complexity: u32,
    pub body_hash: Option<String>,
    pub return_type: Option<TypeRef>,
}
```

### ClassInfo Fields

```rust
pub struct ClassInfo {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub name_col: u32,      // 0-based column of the name identifier
    pub name_end_col: u32,  // 0-based end column of the name identifier
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

> **Don't use `cargo build` directly** for releases. The raw `.wasm` produced by Cargo is a core module, not a component — Cha's host won't load it. `cha plugin build` wraps `cargo build` with the component-encoding step (`wasm-tools component new` + WASI adapter).
>
> If you must use `cargo build` (e.g. for testing during development), run `cha plugin build` once afterwards before reinstalling, otherwise the host loads the previous version.

### WASM Compatibility Cheatsheet

The plugin runs in `wasm32-wasip1` with the WASI Reactor adapter. Some Rust crates do not work in this environment, even if they "compile":

| Crate / API | Status | Notes |
|---|---|---|
| `regex` | ❌ panics at runtime | `Regex::new()` fails inside `wasmtime 44 + reactor` adapter. Use hand-rolled char scanning instead — for typical plugin patterns it's ~50 LOC and safer. |
| `std::time::SystemTime::now()` | ❌ unreliable / panics | WASI clock support varies across hosts. If you need "today's date", expose a `today` `.cha.toml` option instead. |
| `serde_json` | ✅ works | Heavy but no surprises. |
| `tree-sitter` (the Rust crate) | ❌ don't try | Plugins run inside WASM; tree-sitter would need a recursive embedding. Use the `tree_query` host import. |
| Filesystem access | ❌ disabled | `std::fs::read_to_string(&input.path)` returns empty. Use `input.content`. |
| `git` / network | ❌ disabled | No subprocess, no sockets. |

When in doubt: keep dependencies minimal, prefer hand-rolled parsing for small patterns, and pass time/external state in via plugin options.

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

    #[test]
    fn list_options_work() {
        WasmPluginTest::new()
            .source("typescript", "// REVIEW: needs second look")
            .option_list("extra_tags", &["REVIEW"])
            .assert_finding("extended_todo_tag");
    }
}
```

Available option setters:
- `.option(key, value)` — string
- `.option_list(key, &[values])` — list of strings
- `.option_bool(key, true_or_false)`
- `.option_int(key, integer)`
- `.option_float(key, float)`

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
