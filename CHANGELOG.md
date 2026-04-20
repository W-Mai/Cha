# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.10] - 2026-04-21

### Added
- Global `--config <path>` flag for all subcommands — load config from custom file
- `ImportInfo.is_module_decl` field to distinguish module declarations from imports

### Fixed
- Rust `mod` declarations no longer inflate `high_coupling` count

## [1.0.9] - 2026-04-20

### Added
- `cha layers --format html` — interactive architecture diagram with CSS Grid
- Layer violations show file-level evidence (which file includes which)
- Layer violations sorted by instability gap (most severe first)
- Rust `mod` declarations treated as file imports for layer analysis
- Manual layer/module config in `.cha.toml` (`[layers.modules]` + `[[layers.tiers]]`)

## [1.0.8] - 2026-04-20

### Added
- `cha calibrate` command: auto-suggest thresholds from project statistics (P90/P95)
- `cha calibrate --apply` saves to `.cha/calibration.toml`, auto-applied by `cha analyze`
- Finding priority sorting: most severe issues shown first (severity × overshoot × compound)
- Short module names in all output formats (terminal/DSM/dot/mermaid)

### Changed
- DSM output limited to top 25 modules by file count

### Fixed
- Skip parent→child layer violations (reduces lvgl false positives 87→37)

## [1.0.7] - 2026-04-20

### Added
- Module inference rewrite: directory elbow + LCOM4 adaptive split + ICR + TCC quality metrics
- `cha layers --depth N` to override auto-detected directory depth
- `cha layers --format dsm|terminal` output formats
- Composite risk scoring for `long_method`: `risk = lines_ratio × complexity_factor`

### Changed
- Module inference algorithm: replaced Union-Find with directory elbow + LCOM4 + ICR
- `long_method` severity now based on composite risk (Hint/Warning/Error at risk 1/2/4)

### Fixed
- `cha:ignore` directive now covers up to 2 lines before a function
- Fixed corrupted dot output and switched to LR layout for better layer readability

## [1.0.6] - 2026-04-20

### Added
- Language-adaptive thresholds: C/C++ profile with higher defaults (long_method=100, complexity=15, large_file=2000)
- Smart terminal aggregation: findings >5 grouped into summary + top 3 worst, `--all` flag for full listing
- `cha layers` command: infer architectural layers from import dependencies
- `cha layers --format dot|mermaid|json|plantuml` with layered architecture diagram

## [1.0.5] - 2026-04-17

### Fixed
- VS Code extension: download URL corrected (`cha-cli-` prefix), extract path for cargo-dist tarball
- VS Code extension: download with progress bar and cancellation support
- VS Code extension: removed system PATH fallback for reliable self-testing
- `cargo publish` no longer needs `--allow-dirty` (WIT copies tracked in git, `include` in Cargo.toml)

## [1.0.4] - 2026-04-17

### Added
- `cha:set` inline directive: override thresholds per-function/class via comments (`// cha:set rule_name=value`)
- `Finding.actual_value` and `Finding.threshold` fields for post-filter re-evaluation
- `cha lsp` subcommand: start LSP server from unified binary (+3MB)
- `deps --direction in|out|both`: filter edges by direction (who depends on target vs target depends on)
- `deps --format plantuml`: PlantUML output for component and class diagrams
- C OOP false positive filter: removes lazy_class/data_class for structs with cross-file methods
- `.pre-commit-hooks.yaml`: pre-commit framework integration
- `action.yml`: GitHub Action for CI analysis with SARIF upload
- VS Code extension (`vscode-cha/`): cha LSP integration, auto-download binary, esbuild bundle

### Fixed
- `.h` files with C++ constructs now parsed as C++ (content sniffing)
- `class MACRO Name {}` no longer misidentified as function definition
- WIT `Finding` record now includes `actual_value`/`threshold` fields
- `build.rs` auto-copies `wit/plugin.wit` for crates.io packaging
- VS Code extension: esbuild bundle, LICENSE, `.vscodeignore`, publisher ID, homepage

## [0.7.0] - 2026-04-17

### Added
- Dynamic shell completion for `--plugin` via `CompleteEnv` (unstable-dynamic): `eval "$(COMPLETE=zsh cha)"`
- `PluginRegistry::plugin_info()` for runtime plugin discovery with descriptions
- `Plugin` trait unified: `version()`, `description()`, `authors()` with defaults from Cargo.toml
- All 33 builtin plugins now have description text for shell completion
- `completions` subcommand now outputs dynamic completion scripts; shows usage when called without args
- `--strictness` flag: `relaxed` (2x), `default` (1x), `strict` (0.5x), or custom float — scales all numeric thresholds
- Per-language plugin config: `[languages.c.plugins.naming]` overrides in `.cha.toml`
- Builtin C language profile: disables naming, lazy_class, data_class, builder/null_object/strategy pattern by default
- `cha preset list/show` subcommand — display language profiles and plugin rules
- `SourceModel.type_aliases` — unified typedef/type alias tracking across all languages
- C OOP heuristic: associate functions with structs via inheritance chain + same-module matching
- `--exact --detail` now shows only directly matched classes, not parents/children
- C parser `extract_params` now includes pointer info (`Type *`) from AST
- UML class diagrams: `static` functions shown as private (`-`), non-static as public (`+`)

### Changed
- `Config` struct now has `strictness` and `languages` fields (fully backward compatible)
- `get_usize()` applies strictness scaling factor automatically
- `cmd_analyze` refactored into `AnalyzeOpts` + `run_post_analysis()` + `apply_filters()`
- `parse_all_models` returns `(PathBuf, SourceModel)` pairs for correct file-model association

### Fixed
- C/C++ parser: `static` functions now correctly marked `is_exported = false`; header files always exported
- Reduces `large_api_surface` false positives by ~51% and enables accurate `dead_code` detection for C
- `shotgun_surgery`, `divergent_change`, `bus_factor` now use single batch `git log` call instead of per-file — fixes freeze on large repos (lvgl: >2min → 23s)
- C OOP method association resolves typedef aliases for cross-file matching
- `class_dir` prefers struct definitions with fields over forward declarations

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
