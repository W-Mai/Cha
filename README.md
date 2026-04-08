# Cha

<p align="center">
  <img src="static/logo.svg" alt="cha logo" width="160"/>
</p>

<p align="center">
  <strong>察 — Code Health Analyzer</strong>
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

**Cha** (察, "to examine") is a pluggable code smell detection toolkit. It parses source code at the AST level, runs architectural health checks, and reports findings as terminal output, JSON, LLM context, or SARIF.

## ⚡ Quick Start

```bash
# Analyze current directory (recursive, .gitignore aware)
cha analyze

# Analyze specific path with JSON output
cha analyze src/ --format json --fail-on error

# Only analyze changed files (git diff)
cha analyze --diff

# Analyze changes from piped diff (e.g. PR review)
gh pr diff | cha analyze --stdin-diff --fail-on warning

# Run specific plugins only
cha analyze --plugin complexity,naming

# Parse and inspect file structure
cha parse src/

# Generate default config
cha init

# Print JSON Schema for output format
cha schema

# Auto-fix naming convention violations
cha fix src/ --dry-run
```

## 📦 Installation

```bash
git clone https://github.com/W-Mai/Cha.git
cd Cha
cargo build --release
```

Binaries `cha` and `cha-lsp` will be in `target/release/`.

Requires [Rust](https://www.rust-lang.org/tools/install) (edition 2024).

## 🔍 Built-in Plugins

| Plugin | Detects | Category | Severity |
|--------|---------|----------|----------|
| **LengthAnalyzer** | Long methods (>50 lines), large classes, large files | Bloaters | Warning |
| **ComplexityAnalyzer** | High cyclomatic complexity | Bloaters | Warning/Error |
| **DuplicateCodeAnalyzer** | Structural duplication via AST hash (>10 lines) | Dispensables | Warning |
| **CouplingAnalyzer** | Excessive imports / dependencies | Couplers | Warning |
| **NamingAnalyzer** | Too-short names, convention violations | Bloaters | Hint/Warning |
| **DeadCodeAnalyzer** | Unexported / unreferenced code | Dispensables | Hint |
| **ApiSurfaceAnalyzer** | Over-exposed public API (>80% exported) | Couplers | Warning |
| **LayerViolationAnalyzer** | Cross-layer dependency violations | Change Preventers | Error |
| **LongParameterListAnalyzer** | Functions with >5 parameters | Bloaters | Warning |
| **SwitchStatementAnalyzer** | Excessive switch/match arms (>8) | OO Abusers | Warning |
| **MessageChainAnalyzer** | Deep field access chains (a.b.c.d) | Couplers | Warning |
| **PrimitiveObsessionAnalyzer** | Functions with mostly primitive parameter types | Bloaters | Hint |
| **DataClumpsAnalyzer** | Repeated parameter type signatures across functions | Bloaters | Hint |
| **FeatureEnvyAnalyzer** | Methods that reference external objects more than their own | Couplers | Hint |
| **MiddleManAnalyzer** | Classes where most methods only delegate | Couplers | Hint |

Supported languages: TypeScript (.ts/.tsx), Rust (.rs).

## ⚙️ Configuration

Create `.cha.toml` in your project root:

```toml
# Exclude paths from analysis (glob patterns)
exclude = ["*/tests/fixtures/*", "vendor/*"]

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
```

All plugins are enabled by default. Set `enabled = false` to disable.

## 🧩 WASM Plugins

Extend with custom analyzers via WebAssembly Component Model:

```bash
cd examples/wasm-plugin-example
cargo build --target wasm32-wasip1
wasm-tools component new target/wasm32-wasip1/release/example.wasm \
  --adapt wasi_snapshot_preview1.reactor.wasm \
  -o plugin.wasm
```

WIT interface (`wit/plugin.wit`):

```wit
package cha:plugin@0.1.0;

world analyzer {
    export name: func() -> string;
    export analyze: func(input: source-input) -> list<finding>;
}
```

## 💡 LSP Integration

```bash
cha-lsp
```

Provides diagnostics on open/change/save and code action suggestions.

## 🛠️ Development

```bash
# Run all CI checks locally
cargo xtask ci

# Individual steps
cargo xtask build     # Release build
cargo xtask test      # Unit + property + fixture tests
cargo xtask lint      # Clippy + fmt
cargo xtask analyze   # Self-analysis in all formats
cargo xtask lsp-test  # LSP smoke test
```

## 📁 Project Structure

```
cha-core/       Core traits, plugin registry, reporters, WASM runtime
cha-parser/     Tree-sitter parsing (TypeScript, Rust)
cha-cli/        CLI binary (analyze, parse)
cha-lsp/        LSP server binary
xtask/          CI automation (cargo xtask)
wit/            WIT interface for WASM plugins
examples/       Example WASM plugin
static/         Logo and assets
```

## 📄 License

MIT License.
