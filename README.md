# Cha

<p align="center">
  <img src="static/logo.svg" alt="cha logo" width="160"/>
</p>

<p align="center">
  <strong>察 — Code Health Analyzer</strong>
</p>

<p align="center">
  <a href="README.md">English</a> |
  <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="https://github.com/W-Mai/Cha/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/W-Mai/Cha/ci.yml?style=flat-square" alt="CI" />
  </a>
  <a href="https://github.com/W-Mai/Cha/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/W-Mai/Cha?style=flat-square" alt="License" />
  </a>
  <a href="https://github.com/W-Mai/Cha">
    <img src="https://img.shields.io/github/stars/W-Mai/Cha?style=flat-square" alt="Stars" />
  </a>
  <a href="https://github.com/W-Mai/Cha/releases">
    <img src="https://img.shields.io/github/v/release/W-Mai/Cha?style=flat-square" alt="Release" />
  </a>
</p>

**Cha** (察, "to examine") is a pluggable code smell detection toolkit. It parses source code at the AST level via tree-sitter, runs 34 built-in detectors plus user-supplied WASM plugins, and reports findings as terminal output, JSON, LLM context, SARIF, or HTML.

Supported languages: Python (`.py`), TypeScript / TSX (`.ts`, `.tsx`, `.mts`, `.cts`), Rust (`.rs`), Go (`.go`), C (`.c`, `.h`), C++ (`.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx`).

## ⚡ Quick Start

```bash
# Analyze current directory (recursive, .gitignore aware)
cha analyze

# Analyze a path with JSON output, fail CI on any error-severity finding
cha analyze src/ --format json --fail-on error

# Only analyze files changed in the working tree
cha analyze --diff

# Analyze a piped diff (PR review)
gh pr diff | cha analyze --stdin-diff --fail-on warning

# Run specific plugins only
cha analyze --plugin complexity,naming

# Force full re-analysis (skip cache)
cha analyze --no-cache

# Generate baseline of current issues; later, only report new ones
cha baseline
cha analyze --baseline .cha/baseline.json

# Generate HTML report
cha analyze --format html --output report.html

# Inspect parsed file structure (functions, classes, imports)
cha parse src/

# Generate default config / JSON schema
cha init
cha schema

# Auto-fix simple issues (currently: PascalCase rename for naming_convention)
cha fix src/ --dry-run

# Show recent issue trend across the last N commits
cha trend -c 20

# WASM plugin lifecycle
cha plugin new my-plugin
cha plugin build
cha plugin install my_plugin.wasm
cha plugin list
cha plugin remove my_plugin

# Shell completions (fish/bash/zsh/powershell), with dynamic plugin name completion
cha completions fish > ~/.config/fish/completions/cha.fish

# Show built-in language presets and strictness levels
cha preset

# Import / class / call graphs (DOT, JSON, Mermaid, PlantUML, DSM, terminal, HTML)
cha deps --format dot
cha deps --format mermaid --depth dir
cha deps --type classes --filter Plugin --detail --format plantuml
cha deps --type calls --filter analyze --direction in    # who calls analyze?
cha deps --type calls --filter analyze --direction out   # what does analyze call?

# Refactoring hotspots (change frequency × complexity, from git log)
cha hotspot
cha hotspot -c 200 -t 10 --format json

# Infer architectural layers from import dependencies
cha layers
cha layers --format dsm        # dependency structure matrix
cha layers --format mermaid
cha layers --depth 2           # override auto-detected directory depth

# Auto-suggest thresholds from project statistics (P90 = warning, P95 = error)
cha calibrate
cha calibrate --apply          # save to .cha/calibration.toml (auto-loaded by analyze)
```

## ⚡ Performance

Cha uses a two-level cache (L1 in-memory + L2 bincode on disk) with an mtime fast-path so repeat analyses on unchanged files skip parsing entirely.

Historical numbers, measured on 3,201 C files from NuttX RTOS when the cache layer was first introduced (no-cache vs. warm-cache):

| Command | No cache | Warm cache | Speedup |
|---------|----------|------------|---------|
| `analyze` | 5.7s | **3.3s** | 26× |
| `layers` | — | **0.8s** | 16× |
| `deps` | — | **0.9s** | 14× |
| `calibrate` | — | **0.6s** | 22× |

## 📦 Installation

### Shell (macOS / Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.sh | sh
```

### PowerShell (Windows)

```powershell
powershell -c "irm https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.ps1 | iex"
```

### Homebrew

```bash
brew install W-Mai/cellar/cha-cli
```

### From source

```bash
git clone https://github.com/W-Mai/Cha.git
cd Cha
cargo build --release
```

Requires [Rust](https://www.rust-lang.org/tools/install) (edition 2024).

See [cha.to01.icu](https://cha.to01.icu) for all platforms and download options.

## 🔍 Built-in Plugins

34 plugins, 45 smells. A handful of plugins (`length`, `naming`, `error_handling`, `design_pattern`) emit more than one smell from a single detector. Plugins are grouped below by `SmellCategory`, the same grouping used by CLI output, JSON reports, and `--focus`.

All plugins are enabled by default. Disable individually with `enabled = false` under `[plugins.<name>]`. The C language preset turns off `naming`, `lazy_class`, `data_class`, and `design_pattern`.

Defaults below are the values in `Default for <Analyzer>`; every threshold scales by the global `strictness` factor and can be overridden per-plugin in `.cha.toml` or per-item via inline `cha:set` directives.

### Bloaters — code that has grown too large

| Plugin | Smells | Default thresholds | Severity |
|--------|--------|--------------------|----------|
| `length` | `long_method`, `large_class`, `large_file` | `max_function_lines=50`, `max_class_methods=10`, `max_class_lines=200`, `max_file_lines=500`, `complexity_factor_threshold=10.0` | Hint / Warning / Error (scales by how far over the limit) |
| `complexity` | `high_complexity` | `warn_threshold=10`, `error_threshold=20` | Warning / Error |
| `cognitive_complexity` | `cognitive_complexity` | `threshold=15` (penalises nesting depth on top of the basic complexity count) | Warning / Error |
| `long_parameter_list` | `long_parameter_list` | `max_params=5` | Warning |
| `primitive_obsession` | `primitive_obsession` | `min_params=3`, `primitive_ratio=0.8` | Hint |
| `data_clumps` | `data_clumps` | `min_clump_size=3`, `min_occurrences=3` | Hint |
| `naming` | `naming_convention`, `naming_too_short`, `naming_too_long` | `min_name_length=2`, `max_name_length=50` | Hint / Warning |
| `api_surface` | `large_api_surface` | `max_exported_ratio=0.8`, `max_exported_count=20`; for C: `c_max_exported_ratio=1.01`, `c_max_exported_count=30`, `skip_c_headers=true` | Warning |
| `god_class` | `god_class` | `max_external_refs=5` (ATFD — access to foreign data), `min_wmc=47` (weighted method count), `min_tcc=0.33` (tight class cohesion) | Warning |
| `brain_method` | `brain_method` | `min_lines=65`, `min_complexity=4`, `min_external_refs=7` | Warning |

### Couplers — modules that depend too tightly on each other

| Plugin | Smells | Default thresholds | Severity |
|--------|--------|--------------------|----------|
| `coupling` | `high_coupling` | `max_imports=15`; promotes to Error above `2 × max_imports` | Warning / Error |
| `hub_like_dependency` | `hub_like_dependency` | `max_imports=20` | Warning |
| `feature_envy` | `feature_envy` | `min_refs=3`, `external_ratio=0.7` | Hint |
| `middle_man` | `middle_man` | `min_methods=3`, `delegation_ratio=0.5` | Hint |
| `message_chain` | `message_chain` | `max_depth=3` (e.g. `a.b.c.d` triggers) | Warning |
| `inappropriate_intimacy` | `inappropriate_intimacy` | Detects bidirectional imports between two files | Warning |
| `layer_violation` | `layer_violation` | Layers configured via `layers = "domain:0,service:1,..."`; lower-rank may not import higher-rank | Error |
| `async_callback_leak` | `async_callback_leak` | Function signatures leaking raw `JoinHandle` / `Future` / `Channel` types | Hint |

### OO Abusers — object-oriented constructs used incorrectly

| Plugin | Smells | Default thresholds | Severity |
|--------|--------|--------------------|----------|
| `switch_statement` | `switch_statement` | `max_arms=8` (`switch` / `match` / Python `match` / Go `switch`) | Warning |
| `temporary_field` | `temporary_field` | `min_methods=3`, `max_usage_ratio=0.3` | Hint |
| `refused_bequest` | `refused_bequest` | `min_override_ratio=0.5`, `min_methods=3` | Hint |
| `design_pattern` | `strategy_pattern`, `state_pattern`, `builder_pattern`, `null_object_pattern`, `template_method_pattern`, `observer_pattern` | `strategy_min_arms=4`, `state_min_arms=3`, `builder_min_params=7` (or `builder_alt_min_params=5` + `builder_alt_min_optional=3`), `null_object_min_count=3`, `template_min_self_calls=3`, `template_min_methods=4`; type / state field keyword lists configurable | Hint |

### Change Preventers — change in one place forces changes elsewhere

| Plugin | Smells | Default thresholds | Severity |
|--------|--------|--------------------|----------|
| `shotgun_surgery` | `shotgun_surgery` | `min_co_changes=5`, `max_commits=100` (reads `git log`) | Hint |
| `divergent_change` | `divergent_change` | `min_distinct_reasons=4`, `max_commits=50` (reads `git log`) | Hint |

### Dispensables — code that can be removed without losing function

| Plugin | Smells | Default thresholds | Severity |
|--------|--------|--------------------|----------|
| `dead_code` | `dead_code` | Unexported + unreferenced symbols; `entry_points` configurable (defaults include Rust `main`/`tokio_main`, Python `__init__`/`__main__`, Go `init`, C `_start`) | Hint |
| `duplicate_code` | `duplicate_code` | AST-hash duplicate blocks ≥ 10 lines | Warning |
| `comments` | `excessive_comments` | `max_comment_ratio=0.3`, `min_lines=10` | Hint |
| `lazy_class` | `lazy_class` | `max_methods=1`, `max_lines=10` | Hint |
| `data_class` | `data_class` | `min_fields=2` and methods are only field accessors | Hint |
| `speculative_generality` | `speculative_generality` | Interface / trait with ≤ 1 implementation | Hint |
| `todo_tracker` | `todo_comment` | TODO / FIXME / HACK / XXX comments; FIXME promotes to Warning | Hint / Warning |

### Security — risky calls and leaked secrets

| Plugin | Smells | Default thresholds | Severity |
|--------|--------|--------------------|----------|
| `hardcoded_secret` | `hardcoded_secret` | Regex matches against `string_literal` AST nodes; covers API keys, tokens, passwords, private keys, JWTs | Warning |
| `unsafe_api` | `unsafe_api` | Dangerous calls: `eval`, `exec`, `system`, `popen`, `sprintf`, `strcpy`, `strcat`, `gets`, `unsafe`, `innerHTML`, `dangerouslySetInnerHTML` | Warning |
| `error_handling` | `empty_catch`, `unwrap_abuse` | `max_unwraps_per_function=3` for `unwrap()` / `expect()`; empty `catch` / `except` blocks always flagged | Warning |

Every plugin's `Default` impl and `analyze()` body live under [`cha-core/src/plugins/`](cha-core/src/plugins). Run `cha preset` to see built-in language presets and strictness levels, or `cha analyze --plugin <name>` to run a single detector.

## ⚙️ Configuration

Create `.cha.toml` in your project root:

```toml
# Exclude paths from analysis (glob patterns)
exclude = ["*/tests/fixtures/*", "vendor/*"]

# Strictness scales every threshold:
#   relaxed = 2.0×, default = 1.0×, strict = 0.5×, or any custom float (e.g. 0.7)
strictness = "default"

[plugins.length]
enabled = true
max_function_lines = 30
max_class_lines = 200

[plugins.complexity]
warn_threshold = 10
error_threshold = 20

[plugins.coupling]
max_imports = 15

[plugins.layer_violation]
enabled = true
layers = "domain:0,service:1,controller:2"

# Per-language overrides — only the diff from global
[languages.c.plugins.naming]
enabled = false  # C uses snake_case, skip PascalCase check

[languages.c.plugins.length]
max_function_lines = 80

# Tech-debt minutes per severity (used by analyze summary)
[debt_weights]
hint = 5
warning = 15
error = 30
```

### Inline directives

Suppress or relax rules per-item directly in source:

```rust
// cha:ignore                        — suppress all rules for the next item
// cha:ignore long_method            — suppress one rule
// cha:ignore long_method,complexity — suppress multiple
// cha:set long_method=100           — raise the long_method threshold to 100 for the next item
// cha:set threshold=200             — raise the threshold for every threshold-based rule
```

Works with `//`, `#`, and `/* */` comment styles.

## 🧩 WASM Plugins

Custom analyzers ship as WebAssembly Component Model modules.

```bash
cd examples/wasm-plugin-example
cha plugin build
cha plugin install example.wasm
```

Installed `.wasm` files live in `.cha/plugins/` (project-local) or `~/.cha/plugins/` (global). Per-plugin options go in `.cha.toml`:

```toml
[plugins.hardcoded-strings]
SITE_DOMAIN = "example.com"
USER_NAME   = "octocat"
```

### Writing a plugin

`Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
cha-plugin-sdk = { git = "https://github.com/W-Mai/Cha" }
wit-bindgen = "0.55"
```

`src/lib.rs`:

```rust
cha_plugin_sdk::plugin!(MyPlugin);

struct MyPlugin;

impl PluginImpl for MyPlugin {
    fn name() -> String { "my-plugin".into() }
    fn smells() -> Vec<String> { vec!["my_smell".into()] }
    fn analyze(input: AnalysisInput) -> Vec<Finding> { vec![] }
}
```

Four end-to-end examples ship under [`examples/`](examples):

- [`wasm-plugin-example`](examples/wasm-plugin-example) — suspicious function names
- [`wasm-plugin-hardcoded`](examples/wasm-plugin-hardcoded) — hardcoded strings driven by config
- [`wasm-plugin-react-hooks`](examples/wasm-plugin-react-hooks) — React hook rules
- [`wasm-plugin-todo-tracker`](examples/wasm-plugin-todo-tracker) — TODO/FIXME tracker

📖 **[Full Plugin Development Guide](docs/plugin-development.md)**

## 💡 LSP Integration

```bash
cha lsp
```

Implemented capabilities (see `cha-lsp/src/lib.rs`):

- **Diagnostics** — real-time code smell detection on open / change / save
- **Code Actions** — suggested refactorings + Extract Method
- **CodeLens** — complexity, line count, parameter count above each function / class
- **Hover** — markdown quality report card
- **Inlay Hints** — inline `cx:N cog:N NL` annotations
- **Document Symbols** — outline view with ⚠ markers on problematic items
- **Semantic Tokens** — warning modifier on functions / classes with findings
- **Workspace Diagnostics** — full project scan without opening files
- **Progress** — progress notifications during workspace scan

Works with any LSP-compatible editor (VS Code, Neovim, Helix, Zed, Sublime).

## 🔌 Integrations

### Pre-commit

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/W-Mai/Cha
    rev: v1.19.0
    hooks:
      - id: cha-analyze
```

### GitHub Action

```yaml
# .github/workflows/cha.yml
- uses: W-Mai/Cha@v1.19.0
  with:
    fail-on: warning
    upload-sarif: true
```

### VS Code

Install the [Cha extension](https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha) from the Marketplace. It auto-downloads the matching `cha` binary on first launch.

Features: every LSP capability above + auto-update.

## 🛠️ Development

```bash
# All CI checks locally
cargo xtask ci

# Individual steps
cargo xtask build             # Release build
cargo xtask test              # Unit + property + fixture tests
cargo xtask lint              # Clippy + fmt
cargo xtask analyze           # Self-analysis in every output format
cargo xtask lsp-test          # LSP smoke test
cargo xtask plugin-test       # Plugin SDK + macro tests
cargo xtask plugin-e2e        # End-to-end WASM plugin scenarios
cargo xtask integration-test  # CLI integration tests

# Bump workspace version (rewrites every Cargo.toml + Cargo.lock + vscode-cha/package.json)
cargo xtask bump <major|minor|patch>

# Release: tag → wait for release.yml → publish to crates.io
cargo xtask release
```

## 📁 Project Structure

```
cha-core/         Plugin trait, registry, reporters, WASM runtime, query helper
cha-parser/       Tree-sitter parsing for Python, TypeScript, Rust, Go, C, C++
cha-cli/          CLI binary (analyze, parse, deps, layers, hotspot, calibrate, fix, plugin, lsp, …)
cha-lsp/          LSP server library
cha-plugin-sdk/   Guest-side SDK + macro for writing WASM plugins
xtask/            CI / release automation (cargo xtask)
wit/              WIT interface for WASM plugins
examples/         Reference WASM plugins (4)
vscode-cha/       VS Code extension
docs/             Plugin development guide and other long-form docs
static/           Logo and assets
```

## 📄 License

MIT License.
