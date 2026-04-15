# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Dynamic shell completion for `--plugin` via `CompleteEnv` (unstable-dynamic): `eval "$(COMPLETE=zsh cha)"`
- `PluginRegistry::plugin_info()` for runtime plugin discovery with descriptions
- `Plugin` trait unified: `version()`, `description()`, `authors()` with defaults from Cargo.toml
- All 33 builtin plugins now have description text for shell completion
- `completions` subcommand now outputs dynamic completion scripts; shows usage when called without args
- `--strictness` flag: `relaxed` (2x), `default` (1x), `strict` (0.5x), or custom float — scales all numeric thresholds
- Per-language plugin config: `[languages.c.plugins.naming]` overrides in `.cha.toml`
- Builtin C language profile: disables naming, lazy_class, data_class, builder/null_object/strategy pattern by default
- `AnalyzeOpts` struct replaces 8-parameter `cmd_analyze` (eliminates brain_method smell)

### Changed
- `Config` struct now has `strictness` and `languages` fields (fully backward compatible)
- `get_usize()` applies strictness scaling factor automatically
- `cmd_analyze` refactored into `run_post_analysis()` + `apply_filters()`

### Fixed
- C/C++ parser: `static` functions now correctly marked `is_exported = false`; header files always exported
- Reduces `large_api_surface` false positives by ~51% and enables accurate `dead_code` detection for C
- `shotgun_surgery`, `divergent_change`, `bus_factor` now use single batch `git log` call instead of per-file — fixes freeze on large repos (lvgl: >2min → 23s)

## [0.6.2] - 2026-04-15

### Added
- All parser fields implemented for C/C++, Go, Python (zero TODO(parser) remaining)
- C-style struct inheritance detection via first-field type + typedef alias resolution
- `--filter` now shows connected subgraph (children + parent chain, no siblings)
- `--exact` flag for direct-match-only filtering
- `--filter` supports regex patterns
- `--detail` flag for UML class diagrams with fields, types, and methods
- `ClassInfo.field_types` field across all parsers and WIT interface

### Fixed
- C parser: recurse into `#ifdef`/`#if` preprocessor blocks for struct/include detection
- C parser: `typedef struct { ... } Name` now correctly parsed
- Filter traversal: parent chain walk without sibling expansion; fixed infinite loop

## [0.6.1] - 2026-04-14

### Added
- `SourceModel.comments` — parsers now extract comments via tree-sitter for language-aware analysis
- `todo_tracker` now uses parsed comment nodes instead of raw text scanning

### Fixed
- `cha trend` — suppressed git worktree stdout leak; fixed progress bar overlap
- Progress bar spinner now uses braille animation with steady tick
- Extracted `new_progress_bar` helper; added progress bars to `cha deps`
- Unimplemented parser fields marked with `TODO(parser)` comments for self-tracking

## [0.6.0] - 2026-04-14

### Added
- `god_class` plugin — God Class detection (ATFD>5, WMC>=47, TCC<0.33) [Lanza & Marinescu 2006]
- `brain_method` plugin — Brain Method detection (LOC>65, CYCLO>=4, NOAV>7) [Lanza & Marinescu 2006]
- `hub_like_dependency` plugin — detect modules with excessive import fan-out [Arcelli Fontana et al. 2019]
- `error_handling` plugin — detect empty catch blocks and unwrap/expect abuse [Padua & Shang 2018]
- `unstable_dependency` — post-analysis pass using Martin's instability metric I=Ce/(Ca+Ce)
- `cognitive_complexity` plugin — nesting-aware complexity metric, threshold 15 [SonarSource 2017]
- `todo_tracker` plugin — detect leftover TODO/FIXME/HACK/XXX comments
- `unsafe_api` plugin — detect dangerous function calls per language [CWE-676]
- `low_test_ratio` — warn when test code < 50% of production code
- `tangled_change` — detect commits touching unrelated modules [Tornhill 2015]
- `bus_factor` — knowledge distribution risk detection [Nagappan et al. 2008]
- `cha hotspot` subcommand — git change frequency × complexity [Tornhill 2015]

### Fixed
- Duplicate plugin registration bug in `register_advanced_plugins`

## [0.5.2] - 2026-04-13

### Added
- `cha trend` subcommand — analyze recent git commits via worktree, show issue count trend (terminal ASCII + JSON)
- `// cha:ignore` comment directive — suppress findings per function/line, supports `//`, `#`, `--`, `/* */` styles
- `cha deps --type classes` — class hierarchy graph (extends/implements)
- `cha deps --type calls` — function call graph with recursion detection (blue dashed)
- `cha deps --filter <name>` — filter graph to specific class/function

### Fixed
- Cache invalidation now scans all `.cha.toml` files in subdirectories, not just root

## [0.5.1] - 2026-04-12

### Added
- `cha deps --type classes` — class/struct/trait hierarchy graph (extends/implements)
- `cha deps --type calls` — function call graph with recursion detection (blue dashed lines)
- `cha deps --filter <name>` — filter graph to specific class/function
- `FunctionInfo.called_functions` field in parser output and WIT interface

## [0.5.0] - 2026-04-12

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
