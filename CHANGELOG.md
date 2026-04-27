# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- C/C++ parser now extracts function declarations from header files (`void foo(int);` — no body). Previously the parser only recognised `function_definition` nodes at the top level, silently dropping every prototype in a `.h` file. This broke `cha deps --type classes --detail` on C projects (every widget method displayed as private), `leaky_public_signature` (blind to the real public API), and the `c_oop_enrich::tighten_exports` pass (demoted public `.c` implementations whose `.h` declaration didn't parse). Variadic + attribute-macro signatures like `foo(..., ...) LV_FORMAT_ATTRIBUTE(4, 5)` remain an edge case because tree-sitter-c errors on the macro. **Existing `.cha/cache/` entries are stale after this fix and need to be deleted manually** — the cache key hashes `CARGO_PKG_VERSION`, not parser behaviour.

### Changed
- C OOP attribution is now longest-prefix + inheritance-aware: given `struct derived_t { base_t obj; ... }` (first-field embedded base), `derived_do(base_t *obj)` attributes to `derived_t` rather than the base, because the function name's longest matching prefix points at the specific subclass and `derived_t`'s ancestor chain includes `base_t`. Eliminates the previous over-attribution where short prefixes caused base classes to absorb methods that morally belong to subclasses. Large C codebases see base classes drop hundreds of borrowed methods; subclasses now correctly show their own methods in `cha deps --type classes --detail` UML.
- `cha deps --type classes --detail` now uses the project-wide C OOP attribution from `c_oop_enrich` to fill methods on C/C++ UML output. Previously relied on a same-directory heuristic that missed cross-module methods; now picks up methods on shared metaclasses regardless of which file they live in. Also runs enrich on `parse_all_models` so C models read by `deps` see the corrected `method_count` / `has_behavior` / `is_exported`.
- **C OOP cross-file method attribution**: new `cha-cli/src/c_oop_enrich` module runs inside `ProjectIndex::parse` to rewrite `ClassInfo.method_count` / `has_behavior` and tighten `FunctionInfo.is_exported` for C / C++ projects. Uses tokenisation (snake_case, PascalCase, camelCase, acronyms) + typedef alias following to attribute free functions to structs via the universal `foo_t` + `foo_xxx(foo_t *self)` convention. Forward declarations and full definitions of the same struct share attribution. Third-party types declared only in `.c` files (no `.h` declaration) get demoted from exported to internal. Only affects post-analysis index-backed detectors (`anemic_domain_model`, `leaky_public_signature`, etc.); per-file Plugin detectors (`lazy_class`, `data_class`) still see the unenriched model and remain disabled in the C profile.
- Replaces the previous same-file `associate_methods` in `cha-parser::c_lang` (deleted) and the same-directory `c_oop_filter` post-hoc filter in `cha-cli` (deleted) with a single project-wide enrichment pass.

### Added
- `cha analyze --focus <category>` — comma-separated filter keeping only findings whose `SmellCategory` matches one of the listed values (`bloaters`, `oo_abusers`, `change_preventers`, `dispensables`, `couplers`, `security`). Unknown categories warn on stderr instead of crashing. Lets users narrow a noisy analyze run to a single architectural concern.
- `Finding.risk_score: Option<f64>` — composite priority (severity × overshoot × hotspot factor) populated by `prioritize_findings` after analysis. Surfaces *why* a finding ranks where it does in reporter output and JSON/SARIF. Schema regenerated.
- **`leaky_public_signature`** — flags an exported function whose parameters or return type mention a third-party crate's type. Workspace-internal crates (derived from project file paths) and Rust's built-in modules (`std`, `core`, `alloc`, `proc_macro`, …) are filtered out so intra-workspace and prelude types don't fire. Hint severity.
- **`cross_layer_import`** — post-analysis pass that automatically infers project layers from the import graph (Martin's instability) and flags imports crossing boundaries upward. No configuration required; defers to the existing `layer_violation` plugin when the user has written an explicit `[plugins.layer_violation]` config. Warning severity (architectural violation).

## [1.10.0] - 2026-04-25

### Added
- **`god_config`** — flags a `Config`/`Settings`/`Options`/`Context`/`Env`/`AppState`/`Store`-shaped type (exact name or `*Config`/`*Settings`/`*Options` suffix) passed as a parameter to ≥ 10 distinct functions spanning ≥ 3 files. Signals ambient configuration leaking everywhere instead of each caller taking only the fields it actually needs. Hint severity.
- **`circular_abstraction`** — flags two files whose functions call each other's functions in both directions (≥ 2 calls each way). Catches behaviour-level mutual dependency that import-graph cycle detection misses when the callees are re-exported or wrapped. Complements `typed_intimacy` (type flow) with call flow. Hint severity.
- **`parameter_position_inconsistency`** — flags functions where a domain type appears at a different parameter position than the project-wide majority. Requires ≥ 3 usages of the same type across functions and disagreement on position; primitives, unresolved-origin types, mutable-ref out-params (`&mut Vec<_>`), and `self` receivers are skipped. Hint severity.

### Changed
- Internal: `cha-cli/src/project_index.rs` — shared `ProjectIndex` owns parsed models plus derived maps (function_home, class_home, project_type_names, function_by_name). `anemic_domain_model`, `typed_intimacy`, `module_envy`, and `parameter_position_inconsistency` build the index once per analyze call instead of each rebuilding their own copies. No behaviour change; behaviourally identical on self-analyze. Boundary_leak still parses fresh because of a stale-typedef cache bug not yet rooted out.

## [1.9.0] - 2026-04-25

### Added
- **`module_envy`** — flags a function that makes ≥ 3 calls into another file in the project while making ≤ half as many calls within its own file. The function is a "resident" of the wrong module — its body does work that belongs in the envied module. Suppresses test → `common.rs` pairs and calls to conventional helper filenames (`utils`, `helpers`, `shared`, `prelude`, …) where cross-file dependency is idiomatic, not misplaced. Hint severity.
- **`typed_intimacy`** — flags file pairs whose function signatures exchange each other's declared types in both directions. Stronger signal than import-level `inappropriate_intimacy`: the pair literally accepts/returns types defined in each other, indicating they're functionally fused at the type boundary. Emits one finding per side of the pair, listing the shared type names. Hint severity.
- **`async_callback_leak`** — flags a function signature that exposes a raw concurrency primitive (`JoinHandle`, `Future`, `Task`, `Sender`, `Receiver`, `Promise`, `Awaitable`, `Coroutine`, `CancelFunc`, …) in its return type or parameters. Skips launcher-shaped names (`spawn_*`, `launch_*`, `start_*`) where exposing the handle is the function's whole purpose. Hint severity.
- **`anemic_domain_model`** — flags a class that is pure data (≥ 2 fields, no behavior) paired with one or more external service-shaped functions (filename ends in `service`/`manager`/`handler`/`helper`/`util`, or function name starts with a service verb prefix like `process_`/`validate_`/`calculate_`) that take the class as a first parameter. Promotes a `data_class` hint into an architectural finding when there's evidence the paired service owns behavior that should live on the class itself. Hint severity.
- **`test_only_type_in_production`** — warns when production code references a class/struct declared only in test files (mocks, stubs, fixtures). Surfaces test scaffolding bleeding into shipping code. Warning severity.
- **`return_type_leak`** post-analysis finding — dual of `abstraction_boundary_leak`. Detects when a dispatcher fans out to ≥ 3 sibling handlers whose return types are all the same non-local type, surfacing missing Anti-Corruption Layer on the way *out*. lvgl scan identifies thorvg's `TVG_API` leaking through dispatcher boundaries.
- `FunctionInfo.return_type: Option<TypeRef>` — parsers extract the declared return type and resolve its origin through the same imports/type-registry pipeline as parameters. WIT schema grows an optional `return-type` field.
- Container-expression primitive fallback: PEP 585 `dict[K, V]` / `list[T]` / `tuple[...]` resolve to Primitive instead of Unknown, eliminating false positives on Python handlers that return built-in container types.

### Changed
- WIT `function-info` record gains `return-type: option<type-ref>` — **breaking for WASM plugins**, rebuild against the new SDK.
- `cha-cli/src/analyze.rs` — extracted C OOP false-positive filter to `c_oop_filter.rs` and split `run_post_analysis` into git-based and signature-based helpers to keep the orchestrator lean as more post-analysis passes land.

## [1.8.0] - 2026-04-25

### Added
- **`abstraction_boundary_leak`** post-analysis finding — detects dispatcher functions that fan out to ≥ 3 sibling callbacks which all share the same non-local type in corresponding parameter positions. Flagged as a missing Anti-Corruption Layer. lvgl scan shows 11/13 true-positive rate identifying GLAD/SDL/STB/Win32 leaks.
- `FunctionInfo.parameter_types` now carries `TypeRef { name, raw, origin }` where `origin` is `Local | External(module) | Primitive | Unknown`. Each parser resolves origins from file imports: Rust `use_declaration`, TS `import_statement`, Python `import` / `from`, Go `import_spec` with `go.mod` module root lookup, C/C++ primitive seeding.
- Parser normalisation helpers in `cha-parser/src/type_ref.rs` unwrap `&'a mut Vec<Option<T>>`, `[]T`, `List[T]`, `pkg.Type` etc. down to the innermost identifier for import lookup.
- Universal-primitive fallback in resolve (String, PathBuf, HashMap, int, boolean, etc.) so common prelude types without explicit imports don't trip the detector.
- **`unwrap_abuse`** now emits one finding per `.unwrap()` / `.expect(` call site (was: single finding at function name). IDE underlines each call directly.
- **`switch_statement`** now points at the `switch` / `match` keyword inside the function body (was: function name).
- **`message_chain`** now points at the `a.b.c.d` chain expression itself (was: function name). Heuristic text scan, falls back to function name when the chain can't be textually located.

### Changed
- `FunctionInfo.parameter_types` type changed from `Vec<String>` to `Vec<TypeRef>` — **breaking change for WASM plugins and cached SourceModels**. WIT schema adds `type-ref` record and `type-origin` variant. Rebuilding against the new SDK picks up generated types automatically.
- Parsers no longer sort `parameter_types` — declaration order is preserved, fixing latent `.first()`-based C OOP heuristics that silently depended on alphabetical ordering. `data_clumps` plugin now sorts its own key locally.

## [1.7.1] - 2026-04-24

### Fixed
- `cargo xtask release` — `wait_for_workflow` now filters runs by the commit SHA (for ci.yml) and the tag branch (for release.yml), instead of taking the latest run unconditionally. Previously a stale success on an unrelated commit would cause the release flow to skip waiting and publish to crates.io while the new CI was still queued; a stale failure would abort a release that would otherwise pass.

## [1.7.0] - 2026-04-23

### Added
- `cha analyze --top N` flag — show only the N most severe findings (terminal format), complements `--all`
- **Smell-level disable**: `disabled_smells = ["smell_name"]` in `.cha.toml` (global) or under `[languages.<lang>]` (language-scoped). Finer-grained than disabling a whole plugin when it produces multiple smells
- `Plugin::smells()` — plugins declare which `smell_name` values they can produce. Exposed as a WIT export for WASM plugins
- `cha plugin list` now shows each plugin's declared smells
- `cha preset show <lang>` now shows effective disabled smells
- SDK helper `cha_plugin_sdk::is_smell_disabled!(&input.options, "smell_name")` — WASM plugins can skip disabled work proactively

### Changed
- C/C++ builtin profile: `builder_pattern`, `null_object_pattern`, `strategy_pattern`, `data_clumps` are now properly disabled via smell-level config (previously tried — and failed — to disable them by plugin name)
- WIT `analyzer` world gains `smells: func() -> list<string>` export — **breaking change for WASM plugins** (recompile to pick up default impl)

### Fixed
- lvgl-scale improvement: analyze now emits ~1200 fewer false positives because smell-level disables actually take effect

## [1.6.0] - 2026-04-23

### Added
- `Location` now has `start_col`/`end_col` fields — all findings precise to column level
- `FunctionInfo`/`ClassInfo` have `name_col`/`name_end_col` — parser records identifier position
- `ImportInfo` has `col` — import statement column position
- Terminal output shows `file:line:col` when column info available
- SARIF output fills `startColumn`/`endColumn` (1-based per spec)
- LSP diagnostics use precise column range

### Changed
- All 37 builtin plugins now point findings at the function/class name, not the entire body
- Line-scanning plugins (unsafe_api, hardcoded_secret, todo_tracker, error_handling) report exact column
- WIT records gain column fields — `location.start-col`/`end-col`, `function-info.name-col`/`name-end-col`, `class-info.name-col`/`name-end-col`, `import-info.col` — **breaking change for WASM plugins**

## [1.5.0] - 2026-04-22

### Added
- VS Code `cha.disabledPlugins` setting — suppress specific findings via `initializationOptions`
- Hover report card shows actual plugin findings with severity icons
- Coupling/hub_like findings mark import line range precisely

### Changed
- **LSP architecture**: all handlers read from ProjectCache — no per-handler plugin execution
- LSP uses pull-only diagnostics (`textDocument/diagnostic`), removed push duplicates
- CodeLens shows findings count + severity instead of raw parse metrics
- Inlay Hints show findings summary (⚠N or ✓)
- File-level findings (large_file, shotgun_surgery, etc.) mark only line 1

### Fixed
- Duplicate diagnostics (push + pull) in VS Code
- `disabledPlugins` now filters by finding name, not plugin name
- LSP shares `.cha/cache/` with CLI via ProjectCache

## [1.4.2] - 2026-04-22

### Added
- VS Code: auto-detect outdated cha binary — prompt update when version mismatches extension
- VS Code: debug logs in ensureBinary for diagnostics
- VS Code e2e: real VS Code test on 3 platforms (ubuntu/macos/windows) with sinon stub for user Download click

### Fixed
- SDK macros: include build.rs in package
- VS Code: Windows download (.zip + PowerShell + .exe)
- VS Code: exclude test files from .vsix via .vscodeignore
- CI: vscode e2e set continue-on-error for network flakiness

## [1.4.1] - 2026-04-21

### Added
- VS Code extension CI: `vsce package` validation + download e2e test on GitHub Actions
- Download e2e test imports actual extension code (shared `download.ts` module)

### Fixed
- Windows binary download: use `.zip` + PowerShell extraction + `.exe` binary name

## [1.4.0] - 2026-04-21

### Added
- **LSP Semantic Tokens**: highlight functions/classes with warning modifier based on findings
- **LSP Workspace Diagnostics**: full project analysis without opening files
- **LSP textDocument/diagnostic**: pull-based diagnostics per file
- **LSP Progress**: progress notification during workspace diagnostics scan

## [1.3.0] - 2026-04-21

### Added
- **LSP Document Symbols**: outline view with ⚠ markers based on actual findings severity
- **LSP**: Document Symbols ⚠ markers now respect `.cha.toml` thresholds (no hardcoded values)

### Changed
- Upgraded wasmtime 43 → 44
- Include tests in cha-core crate package (eliminates publish warnings)

## [1.2.0] - 2026-04-21

### Added
- **LSP CodeLens**: show complexity, cognitive, lines, params above every function/class
- **LSP Hover**: detailed quality report card on hover (markdown table)
- **LSP Inlay Hints**: inline cx/cog/lines annotations at end of function definitions

## [1.1.0] - 2026-04-21

### Added
- Cache v2: bincode serialization + per-file parse cache + mtime fast-path
- L1 in-memory parse cache — zero disk I/O for repeated access within same process
- Cached imports in meta for instant `unstable_dependency` analysis
- `ProjectCache` with L1/L2 architecture shared across analyze/layers/deps/calibrate

### Changed
- **Performance**: `cha analyze` 26x faster on warm cache (87s → 3.3s on 3201 files)
- **Performance**: `cha layers` 16x faster (13s → 0.8s)
- **Performance**: `cha deps` 14x faster (13s → 0.9s)
- **Performance**: `cha calibrate` 22x faster (13s → 0.6s)

### Fixed
- O(n²) algorithm in `unstable_dependency` / `compute_afferent` replaced with HashMap O(1) lookup
- Findings cache wiped by duplicate `ProjectCache::open` in post-analysis
- Cache invalidation now includes cha binary version (upgrade = auto-invalidate)
- Skip `filter_c_oop_false_positives` when no lazy_class/data_class findings exist

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
