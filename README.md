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

**Cha** (察, "to examine") is a pluggable code smell detection toolkit written in Rust. It parses source code via Tree-sitter, runs a suite of architectural health checks, and reports findings as terminal output, JSON, LLM context, or SARIF for CI integration.

## 📖 Table of Contents

- [Core Features](#-core-features)
- [Quick Start](#-quick-start)
- [Installation](#-installation)
- [Built-in Plugins](#-built-in-plugins)
- [Configuration](#-configuration)
- [WASM Plugins](#-wasm-plugins)
- [LSP Integration](#-lsp-integration)
- [Project Structure](#-project-structure)
- [License](#-license)

## 🚀 Core Features

- **🌳 Tree-sitter Parsing**: AST-level analysis for TypeScript (.ts/.tsx) and Rust (.rs), with per-function complexity, body hashing, and export detection.
- **🔌 8 Built-in Plugins**: Length, complexity, duplication, coupling, naming, dead code, API surface, and layer violation detection.
- **🧩 WASM Plugin System**: Extend with custom analyzers via WebAssembly Component Model (wasmtime + wit-bindgen), fully sandboxed.
- **📊 Multi-format Output**: Terminal (human-readable), JSON, LLM context, and SARIF for CI/CD pipelines.
- **🚦 CI/CD Ready**: `--fail-on hint|warning|error` exit codes + `--diff` for incremental analysis on changed files only.
- **💡 LSP Server**: Real-time diagnostics and code action suggestions in your editor.

## ⚡ Quick Start

```bash
# Analyze current directory
cha analyze .

# JSON output with error-level exit code
cha analyze src/ --format json --fail-on error

# Only analyze changed files (git diff)
cha analyze --diff

# Parse a single file
cha parse src/main.rs
```

## 📦 Installation

### Build from Source

#### Prerequisites
- [Rust Toolchain](https://www.rust-lang.org/tools/install) (edition 2024)

#### Steps

```bash
git clone https://github.com/W-Mai/Cha.git
cd Cha
cargo build --release
```

The binaries `cha` and `cha-lsp` will be in `target/release/`.

## 🔍 Built-in Plugins

| Plugin | Detects | Severity |
|--------|---------|----------|
| **LengthAnalyzer** | Long methods, large classes, large files | Warning |
| **ComplexityAnalyzer** | High cyclomatic complexity | Warning/Error |
| **DuplicateCodeAnalyzer** | Structural duplication via AST hash | Warning |
| **CouplingAnalyzer** | Excessive imports / dependencies | Warning |
| **NamingAnalyzer** | Too-short names, convention violations | Hint |
| **DeadCodeAnalyzer** | Unexported / unreferenced code | Hint |
| **ApiSurfaceAnalyzer** | Over-exposed public API (>80% exported) | Warning |
| **LayerViolationAnalyzer** | Cross-layer dependency violations | Error |

## ⚙️ Configuration

Create a `.cha.toml` in your project root:

```toml
[plugins.length]
enabled = true
max_method_lines = 30
max_class_lines = 300

[plugins.complexity]
enabled = true
threshold = 15

[plugins.layer_violation]
enabled = true
layers = ["domain", "application", "infrastructure"]
```

## 🧩 WASM Plugins

Write custom analyzers in any language that compiles to WASM Component Model:

```bash
cd examples/wasm-plugin-example
cargo build --target wasm32-wasip1
wasm-tools component new target/wasm32-wasip1/release/example.wasm \
  --adapt wasi_snapshot_preview1.reactor.wasm \
  -o plugin.wasm
```

The WIT interface (`wit/plugin.wit`):

```wit
package cha:plugin@0.1.0;

world analyzer {
    export name: func() -> string;
    export analyze: func(input: source-input) -> list<finding>;
}
```

## 💡 LSP Integration

Run the language server for real-time diagnostics:

```bash
cha-lsp
```

Provides:
- Diagnostics on open / change / save
- Code actions with suggested refactorings

## 📁 Project Structure

```
cha-core/       Core traits, plugin registry, reporters, WASM runtime
cha-parser/     Tree-sitter parsing layer (TypeScript, Rust)
cha-cli/        CLI binary (analyze, parse subcommands)
cha-lsp/        LSP server binary
wit/            WIT interface for WASM plugins
examples/       Example WASM plugin
static/         Logo and assets
```

## 📄 License

This project is open source and available under the **MIT License**.
