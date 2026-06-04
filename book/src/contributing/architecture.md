# Architecture

Cha is a Rust workspace with seven crates. The dependency direction is fixed: `cha-core` does not depend on `cha-parser` (it only touches `cha-parser`'s output via the traits in [`cha-core/src/plugin.rs`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/plugin.rs)), `cha-cli` depends on `cha-core`, and `cha-plugin-sdk` depends on neither. Don't reverse this direction.

## Crate map

```text
                ┌──────────┐
                │   xtask  │  ci/release automation
                └─────┬────┘
                      │
   ┌──────────┐   ┌───▼────────┐   ┌──────────┐
   │ cha-cli  │──▶│ cha-core   │◀──│ cha-lsp  │
   │ (binary) │   │ (analysis) │   │ (server) │
   └──────────┘   └─────▲──────┘   └──────────┘
                        │
                  ┌─────┴──────┐
                  │ cha-parser │  tree-sitter wrappers
                  └────────────┘

                ┌────────────────────┐
                │  cha-plugin-sdk    │  guest-side, no host deps
                └────────────────────┘
                       (WASM)
```

| Crate | Lives in | Owns |
|---|---|---|
| `cha-core` | [`cha-core/`](https://github.com/W-Mai/Cha/tree/main/cha-core) | `Plugin` trait, `Finding` / `SourceModel` / `SymbolIndex` model, registry, reporters (terminal/JSON/SARIF/HTML/LLM), WASM runtime, two-level cache. |
| `cha-parser` | [`cha-parser/`](https://github.com/W-Mai/Cha/tree/main/cha-parser) | Tree-sitter parsers for Python, TypeScript / TSX, Rust, Go, C, C++. Produces `SourceModel` and `SymbolIndex`. |
| `cha-cli` | [`cha-cli/`](https://github.com/W-Mai/Cha/tree/main/cha-cli) | Binary. Subcommands: `analyze`, `parse`, `baseline`, `fix`, `deps`, `layers`, `hotspot`, `trend`, `calibrate`, `preset`, `plugin`, `lsp`, etc. |
| `cha-lsp` | [`cha-lsp/`](https://github.com/W-Mai/Cha/tree/main/cha-lsp) | LSP server library + binary entry. Diagnostics, code actions, code lens, hover, inlay hints, semantic tokens, workspace diagnostics. |
| `cha-plugin-sdk` | [`cha-plugin-sdk/`](https://github.com/W-Mai/Cha/tree/main/cha-plugin-sdk) | Guest-side library + `plugin!` macro. Compiles to `wasm32-wasip2`. No `cha-core` dependency. |
| `xtask` | [`xtask/`](https://github.com/W-Mai/Cha/tree/main/xtask) | `cargo xtask` automation: `ci`, `test`, `lint`, `analyze`, `bump`, `release`, `publish`, `docgen-cli`, `docs-check`, `i18n-check`. |
| `vscode-cha` | [`vscode-cha/`](https://github.com/W-Mai/Cha/tree/main/vscode-cha) | VS Code extension. Auto-downloads matching `cha` binary on first launch. |

## Data flow

```text
source files ──▶ cha-parser ──▶ SourceModel ──┐
                                              ├──▶ Plugin::analyze ──▶ Vec<Finding>
                                config TOML ──┤
                                              └──▶ caches (L1 mem + L2 bincode on disk)
```

`SourceModel` is the single shared format. Every plugin sees the same `&AnalysisContext { file, model, config }`. The model is parsed once, hashed into the cache key, and shared across all plugin invocations on the same file.

WASM plugins go through one extra hop: a host adapter in `cha-core::wasm` serialises `AnalysisInput` (a subset of `AnalysisContext` that fits the [WIT interface](https://github.com/W-Mai/Cha/blob/main/wit/cha-plugin.wit)) and crosses the WASM boundary. Inside the guest, [`cha-plugin-sdk`](https://github.com/W-Mai/Cha/tree/main/cha-plugin-sdk) decodes it back into idiomatic Rust types.

## The `Plugin` trait

Built-in detectors implement [`cha_core::Plugin`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/plugin.rs):

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn smells(&self) -> Vec<String>;
    fn description(&self) -> &str;
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding>;
}
```

WASM plugins implement [`cha_plugin_sdk::PluginImpl`](https://github.com/W-Mai/Cha/blob/main/cha-plugin-sdk/src/lib.rs) — a parallel trait with the same shape, modulo string instead of `&str` returns (WIT requirement). The host bridge in `cha-core::wasm` lets a `PluginImpl` impl participate in the same registry as native ones.

## Caching

Two layers, both in `cha-core::cache`:

- **L1**: in-memory `DashMap<PathBuf, CachedResult>`. Lifetime: a single `cha analyze` invocation.
- **L2**: bincode files under `.cha/cache/`. Cache key is `(file mtime, file size, plugin set hash, config hash)`. mtime fast-path skips parsing entirely when nothing's moved.

Plugin set hash includes installed `.wasm` files — installing or reinstalling a plugin invalidates anything that plugin touched, automatically.

## When to extend each crate

| You want to... | Touch |
|---|---|
| Add a built-in smell | `cha-core/src/plugins/` + register in `cha-core/src/registry.rs` |
| Support a new language | `cha-parser/src/<lang>.rs` + map in `cha-parser/src/lib.rs` |
| Add a CLI subcommand | `cha-cli/src/<subcommand>.rs` + wire in `cha-cli/src/main.rs` |
| Expose new SDK functionality to WASM plugins | Update [`wit/cha-plugin.wit`](https://github.com/W-Mai/Cha/blob/main/wit/cha-plugin.wit), regenerate bindings, implement host adapter in `cha-core/src/wasm.rs`, expose in `cha-plugin-sdk/src/lib.rs` |
| Add an LSP capability | `cha-lsp/src/lib.rs` |

## See also

- [Writing a smell](./writing-a-smell.md)
- [Plugin development](../plugins/development.md) (host-side trait + WASM SDK)
- [`Plugin`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/plugin.rs) and [`PluginImpl`](https://github.com/W-Mai/Cha/blob/main/cha-plugin-sdk/src/lib.rs) source
