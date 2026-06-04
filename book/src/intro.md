# Introduction

<p align="center">
  <img src="https://raw.githubusercontent.com/W-Mai/Cha/main/static/logo.svg" alt="cha logo" width="160"/>
</p>

<p align="center">
  <strong>察 — Code Health Analyzer</strong>
</p>

<p align="center">
  <a href="https://github.com/W-Mai/Cha/actions"><img src="https://img.shields.io/github/actions/workflow/status/W-Mai/Cha/ci.yml?style=flat-square" alt="CI" /></a>
  <a href="https://github.com/W-Mai/Cha/blob/main/LICENSE"><img src="https://img.shields.io/github/license/W-Mai/Cha?style=flat-square" alt="License" /></a>
  <a href="https://github.com/W-Mai/Cha"><img src="https://img.shields.io/github/stars/W-Mai/Cha?style=flat-square" alt="Stars" /></a>
  <a href="https://github.com/W-Mai/Cha/releases"><img src="https://img.shields.io/github/v/release/W-Mai/Cha?style=flat-square" alt="Release" /></a>
</p>

**Cha** (察, "to examine") is a pluggable code smell detection toolkit. It parses source code at the AST level via tree-sitter, runs 34 built-in detectors plus user-supplied WASM plugins, and reports findings as terminal output, JSON, LLM context, SARIF, or HTML.

Supported languages: Python (`.py`), TypeScript / TSX (`.ts`, `.tsx`, `.mts`, `.cts`), Rust (`.rs`), Go (`.go`), C (`.c`, `.h`), C++ (`.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx`).

## What you'll find here

- **[Install](./install.md)** — get the `cha` binary on your machine.
- **[Quick Start](./quick-start/cli.md)** — typical workflows in five minutes.
- **[Configuration](./configuration/overview.md)** — `.cha.toml`, strictness, inline directives.
- **[Smells](./plugins/reference.md)** — every built-in detector, what triggers it, how to tune it.
- **[Plugin development](./plugins/development.md)** — author your own WASM plugins.
- **[CLI](./cli/index.md)** — every subcommand documented.
- **[LSP integration](./lsp/overview.md)** — wire Cha into your editor.
- **[Cookbook](./recipes/index.md)** — task-oriented recipes.

## Status

Cha is pre-1.0 — the core engine is stable and self-tested, but configuration formats and CLI surface may evolve. The CHANGELOG lists every breaking change.
