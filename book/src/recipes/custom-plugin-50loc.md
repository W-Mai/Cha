# Custom plugin in 50 lines

A worked example: detect functions whose names contain `tmp`, `temp`, or `xxx`. Project-specific naming hygiene that no built-in detector covers.

The full `cha plugin new` scaffold is bigger than what we need. This recipe strips it to 50 lines of meaningful Rust + the necessary `Cargo.toml`. For the structured tour see [Plugin development](../plugins/development.md).

## Scaffold

```bash
cha plugin new no-tmp-names
cd no-tmp-names
```

`Cargo.toml` (the scaffold writes a working version; what matters):

```toml
[package]
name = "no-tmp-names"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
cha-plugin-sdk = { git = "https://github.com/W-Mai/Cha" }
wit-bindgen = "0.55"
```

## The plugin

`src/lib.rs`:

```rust
use cha_plugin_sdk::{plugin, AnalysisInput, Finding, PluginImpl, Severity};

plugin!(NoTmpNames);

struct NoTmpNames;

const FORBIDDEN: &[&str] = &["tmp", "temp", "xxx"];

impl PluginImpl for NoTmpNames {
    fn name() -> String {
        "no-tmp-names".into()
    }

    fn smells() -> Vec<String> {
        vec!["tmp_named_function".into()]
    }

    fn analyze(input: AnalysisInput) -> Vec<Finding> {
        let mut findings = Vec::new();
        for f in &input.model.functions {
            let lower = f.name.to_lowercase();
            if FORBIDDEN.iter().any(|bad| lower.contains(bad)) {
                findings.push(Finding {
                    smell: "tmp_named_function".into(),
                    severity: Severity::Hint,
                    line: f.start_line,
                    column: f.name_col + 1,           // 1-based
                    end_line: Some(f.start_line),
                    end_column: Some(f.name_end_col + 1),
                    message: format!(
                        "Function `{}` is named like temporary scaffolding — give it the name it deserves before merge.",
                        f.name
                    ),
                    suggestion: None,
                });
            }
        }
        findings
    }
}
```

That's the whole plugin: trait `PluginImpl` with three methods, one loop. No state, no async, no `Result` ceremony.

## Build and install

```bash
cha plugin build              # compiles to target/wasm32-wasip2/release/no_tmp_names.wasm
cha plugin install no_tmp_names.wasm
```

`install` copies the artifact to `.cha/plugins/` (project-local). Use `--global` for `~/.cha/plugins/`.

## Run it

```bash
cha analyze --plugin no-tmp-names src/
```

Filter to one plugin while iterating; the next `cha analyze` (no `--plugin`) runs everything plus your new plugin together.

## Iterate

Edit `src/lib.rs`, then:

```bash
cha plugin build
cha plugin install no_tmp_names.wasm    # overwrites the previous .wasm
cha analyze --plugin no-tmp-names src/
```

Cache invalidation is automatic — installing a new `.wasm` invalidates the cached findings for files that plugin touched.

## What's available inside `analyze`

`AnalysisInput` exposes:

- `input.path` — path of the file being analysed.
- `input.model` — the [SourceModel](../plugins/development.md#functioninfo-fields) with parsed functions, classes, imports, comments.
- `input.options` — values from `[plugins.no-tmp-names]` in `.cha.toml`.

Project-wide queries (callers, type origins, file count) live in `cha_plugin_sdk::project_query`. Tree-sitter S-expression queries live in `cha_plugin_sdk::tree_query`. The full surface is [Plugin development](../plugins/development.md).

## See also

- [Plugin development](../plugins/development.md) — full reference.
- [`examples/`](https://github.com/W-Mai/Cha/tree/main/examples) — four end-to-end plugins, including a TODO tracker and a React hooks linter.
- [`cha plugin`](../cli/plugin.md)
