# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `cha deps` subcommand — import dependency graph with `--format dot|json|mermaid`, `--depth file|dir`, cycle detection with red highlighting
- Go language support (.go) — functions, structs, interfaces, imports, complexity, chain depth
- C language support (.c/.h) — functions, structs, includes, complexity
- C++ language support (.cpp/.cc/.cxx/.hpp/.hxx) — functions, classes, includes, complexity
- Health scores in JSON output (`health_scores` field) and SARIF output (`properties.health_scores`)
- `[debt_weights]` config section in `.cha.toml` — customize remediation time per severity (hint/warning/error)
- Plugin-level parallel analysis via rayon `par_iter`

### Fixed
- HTML report: show only ±5 context lines around findings instead of full file source, collapse file sections by default

## [0.4.0] - 2026-04-11

### Added
- Tech debt summary in terminal output: total estimated remediation time + grade distribution
- `--format html` — self-contained HTML report with dark theme, source code highlighting, health scores, and collapsible file sections
- `--output <path>` flag to write report to file
- `hardcoded_secret` plugin — detects API keys, tokens, passwords, private keys, JWTs in source code
- `SmellCategory::Security` variant for security-related findings

## [0.3.0] - 2026-04-10

### Added
- Incremental analysis cache (`.cha/cache/`) — skips unchanged files, ~70x speedup on warm runs
- `--no-cache` flag to force full re-analysis
- Cache auto-invalidates when `.cha.toml` or plugins change
- `cha baseline` — generate a baseline file of current findings, suppress known issues
- `--baseline <path>` flag on `cha analyze` to only report new findings
- Code health scores (A–F) per file in terminal output, based on issue density and severity

## [0.2.0] - 2026-04-10

### Added
- Python language support (.py) — functions, classes, imports, complexity, chain depth, delegating detection

### Fixed
- `xtask bump` now dynamically scans all Cargo.toml files instead of hardcoded paths, and refreshes all Cargo.lock files
- Duplicate `PythonParser` import in cha-parser
- `cha-lsp/Cargo.toml` version not updated by `xtask bump`

## [0.1.1] - 2026-04-10

### Added
- `cha completions <shell>` — generate shell completion scripts (bash/zsh/fish/powershell); auto-installed via Homebrew

### Fixed
- `cha plugin new` hint now shows `cha plugin build` instead of `cargo build`, and uses correct underscore filename
- WASM plugin e2e test: plugin dir detection when `cha plugin new` uses cwd directly
- Unused `Path` import in `cha-plugin-sdk` test-utils

### Changed
- `cha-lsp`: marked `publish = false`, not distributed via crates.io
- `xtask`: refactored `cmd_publish`/`cmd_bump` to reduce complexity

## [0.1.0] - 2026-04-10

### Added

#### Core Analysis
- 25 built-in code smell plugins covering Bloaters, Couplers, OO Abusers, Change Preventers, and Dispensables
- 9 new plugins: TemporaryField, SpeculativeGenerality, RefusedBequest, ShotgunSurgery, DivergentChange, LazyClass, DataClass, MiddleMan, FeatureEnvy
- DesignPatternAdvisor: suggests Strategy, State, Builder, Null Object, Template Method, Observer patterns
- TypeScript and Rust AST parsing via Tree-sitter
- Structural duplication detection via AST hash

#### WASM Plugin System
- WIT interface with full model fields (`FunctionInfo`, `ClassInfo`) and typed `option-value` variant
- `cha-plugin-sdk` crate: zero-config plugin development — no WIT file needed, `plugin!` macro embeds WIT at compile time
- `cha plugin new/build/install/list/remove` CLI subcommands
- Auto-conversion of WASM binary to WASM Component in `cha plugin build`
- `test-utils` feature: `WasmPluginTest` builder for plugin unit testing
- Plugin metadata (version, description, authors) auto-filled from plugin's `Cargo.toml`
- Config options passed from `.cha.toml` to plugins as typed `OptionValue`

#### CLI
- `cha analyze` — recursive analysis with `.gitignore` awareness, `--diff`, `--stdin-diff`, `--plugin` filter
- `cha parse` — inspect AST structure
- `cha init` — generate default config
- `cha fix` — auto-fix naming violations
- `cha schema` — print JSON Schema for output format
- Output formats: terminal, JSON, SARIF, LLM context
- `--fail-on` exit code control

#### LSP
- Real-time diagnostics on open/change/save
- Code action suggestions

#### Tooling
- `cargo xtask ci/build/test/lint/analyze/lsp-test/plugin-test/plugin-e2e`
- `cargo xtask bump <major|minor|patch>` — version bump across all crates
- `cargo xtask publish [--dry-run]` — publish to crates.io in topological order
- cargo-dist: multi-platform binaries (macOS/Linux/Windows), shell/powershell/homebrew/msi installers
- oranda: project website with release artifacts

[Unreleased]: https://github.com/W-Mai/Cha/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/W-Mai/Cha/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/W-Mai/Cha/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/W-Mai/Cha/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/W-Mai/Cha/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/W-Mai/Cha/releases/tag/v0.1.0
