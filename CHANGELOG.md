# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.20.0] - 2026-06-05

🎉 **First minor since v1.0** — and the first release with a proper home on the web. <https://cha.to01.icu> is now a real product page, not just a README dump. Star the repo, share the link, and tell your linter Cha said hi.

The analyzer itself didn't move — every `--version`-visible behavior is identical to v1.19.0. What landed is the documentation, presentation, and integration story: somewhere we can actually point new users.

### Added

- **Bilingual documentation site at <https://cha.to01.icu>** — landing page (oranda) plus a full mdbook tree at `/book/` (English) and `/book/zh-CN/` (中文). 60+ pages covering install, quick-start, every CLI subcommand, every output format, LSP integration for VS Code / Helix / Neovim / Zed, configuration reference, JSON Schema, plugin development, six worked recipes (migrate from clippy, monorepo CI, suppress in legacy code, custom plugin in 50 lines, calibrate to your codebase, baseline workflow), three contributor guides (architecture, writing a smell, releasing), and an academic-references page tracing every smell back to its source.
- **Cookbook recipes** — six task-oriented walkthroughs: `migrate-from-clippy`, `monorepo-ci`, `suppress-legacy`, `custom-plugin-50loc`, `calibrate`, `baseline`. Each starts with a problem statement and ends with copy-pasteable commands. Authored in English and Chinese (the Chinese versions are written natively, not translated, so the prose style matches the rest of zh-CN).
- **Contributor guides** — `contributing/architecture.md` documents the seven-crate workspace and data flow with mermaid diagrams; `writing-a-smell.md` walks through `MiddleManAnalyzer` as the worked example; `releasing.md` is the runbook for `cargo xtask bump` → `release`.
- **Configuration reference** — every `.cha.toml` key, grouped by plugin, with thresholds and defaults sourced from `cha-core/src/plugins/`. JSON Schema reference page documenting `cha schema` output and how to wire it into IDEs.
- **Bilingual landing page (`LANDING.md`)** — hero with logo + tagline + three CTAs, six feature cards (detectors, WASM SDK, LSP, git-aware analysis, output formats, two-level cache), 30-second get-started block, smell-category table, and editor integration links. Replaces the previous "README dumped into the homepage" experience.
- **CJK-aware search via Pagefind 1.4** — replaces mdbook's bundled elasticlunr (which silently dropped non-ASCII tokens, leaving the zh-CN tree un-searchable). Press `s` or `/` on any docs page to open a modal that indexes both language trees.
- **Per-page social meta + branded OG card** — every page emits `og:title` / `og:image` / `twitter:card` so Slack / iMessage / 微信 link previews show a real card instead of a 32×32 favicon. The 1200×628 card embeds the Cha logo and is regenerable via `python3 static/gen_og_card.py`.
- **Custom 404 page** — site-wide warm-themed 404 with the Cha logo and shortcuts back into the live parts of the site, replacing GitHub Pages' generic gray 404.
- **Language switcher in mdbook header** — the toolbar globe icon switches between EN and zh-CN trees and remembers per-page equivalents.
- **`xtask docs-check`** — verifies every page referenced in `book/src(-zh-CN)/SUMMARY.md` actually exists; runs as part of `cargo xtask ci` so a broken SUMMARY can't ship.
- **`xtask i18n-check`** — flags zh-CN pages whose git ctime trails their English counterpart, surfacing translation drift.
- **`xtask docgen-cli`** — generates `book/src/reference/cli-manual.md` from `cha help-markdown` so the CLI manual page in docs always matches the binary's actual `--help`. Hidden `cha help-markdown` subcommand added for this.
- **VS Code extension page** — full inventory of what each LSP capability looks like inside VS Code (wavy underlines, lightbulbs, code-lens overlays, inlay hints, hover cards, semantic-token modifier, status-bar workspace scan progress) plus the `cha.disabledPlugins` setting documentation.

### Fixed

- **tree-sitter S-expression query link** — the link in `docs/plugin-development.md` and the zh-CN translation pointed at `/syntax-highlighting/queries`, which 404s on the current tree-sitter docs site. Updated to `/using-parsers/queries/`.
- **`docs/plugin-development.md` `FunctionInfo` / `ClassInfo` field tables** — were bare struct dumps; now per-field semantic tables (type + what each field actually drives) for both languages.
- **README plugin table** — every entry now has an anchor link into `docs/plugins.md`'s detailed description, in both `README.md` and `README.zh-CN.md`. The previous `UnstableDependency` row (which never matched a real detector) was removed and `async_callback_leak` added.

### Changed

- **Project homepage** — `https://cha.to01.icu` is the canonical entry point. README still works on github.com but now points at the docs site for anything beyond the quick-start.
- **CI** — the `Web` workflow builds the EN tree via oranda, re-builds the zh-CN tree via mdbook with `MDBOOK_BOOK__SRC=src-zh-CN`, indexes both with Pagefind, and restores `public/CNAME` + drops `public/404.html` after oranda's clean. Deploys via `JamesIves/github-pages-deploy-action@v4.6.4`.

### Notes for upgraders

Nothing to do. `cha analyze` produces the same output, `.cha.toml` accepts the same keys, every CLI flag still works. This is a documentation release: the binary moves from "had a single README" to "had a 60-page bilingual docs site" without changing anything you'd notice from the terminal.

If you want to celebrate by reading something other than `--help`: <https://cha.to01.icu>.

## [1.19.0] - 2026-05-22

Hardcoded thresholds and keyword lists become plugin config; `cha fix` stops hardcoding smell names; the last two text-scanning detectors switch to AST queries.

### Added
- **`Plugin::try_fix(finding, ctx) -> Option<Patch>`** — every plugin can now contribute auto-fixes. `cha fix` walks all enabled plugins and asks each one. Adding fix support for a new smell is one trait override, not a host-side `if smell_name == ...` patch.
- **`cha_core::Patch` / `cha_core::TextEdit`** — public byte-range edit types for plugin authors. Edits within a single finding are applied in reverse byte-offset order.
- **`DesignPatternAdvisor` config** — 8 magic-number thresholds (`strategy_min_arms`, `state_min_arms`, `builder_min_params`, `builder_alt_min_params`, `builder_alt_min_optional`, `null_object_min_count`, `template_min_self_calls`, `template_min_methods`) and 2 keyword lists (`type_field_keywords`, `state_field_keywords`) are now overridable via `[plugins.design_pattern]`.
- **`GodClassAnalyzer::min_tcc`** — Tight Class Cohesion threshold (Lanza-Marinescu's 1/3) is configurable.
- **`cache.rs` walker skip-dirs** — extended from `{target, node_modules, dist}` to also skip `build`, `out`, `__pycache__`, `venv`, `.venv`, `vendor`. Reduces unnecessary `.cha.toml` discovery work in polyglot repos.

### Fixed
- **`switch_statement` / `message_chain`**: replaced bespoke text scanners (`find_switch_keyword`, `walk_chain` and ~140 lines of hand-rolled tokenization) with tree-sitter queries against `switch_statement` / `match_expression` / `field_expression` / `member_expression` / `selector_expression`. Smell counts unchanged, but keyword positions are now sourced from the AST. No more false positives on keywords inside strings.
- **`inappropriate_intimacy` import resolution**: extension probe expanded from `{.ts, .tsx, .rs}` to also include `.py`, `.go`, `.cpp`, `.cc`, `.cxx`, `.c`, `.h`, `.hpp`, `.hxx`, `.js`, `.jsx`, `.mts`, `.cts`. Sibling-file lookups in non-JS/Rust projects no longer silently fail.
- **`calibrate.rs` table rendering**: 3 metric labels were hardcoded across 3 separate `println` blocks. Adding a calibration metric now means one entry in a `(label, samples)` array.

### Changed
- **`cha fix`** delegates to `Plugin::try_fix` for every finding — no more `filter(|f| f.smell_name == "naming_convention")` on the host. Existing behavior preserved (NamingAnalyzer fixes `naming_convention` PascalCase violations); other plugins return `None` until they opt in.

## [1.18.0] - 2026-05-22

Built-in detectors now use AST queries instead of text scanning. Several core plugins previously did substring matches that misfired on strings, comments, and unrelated identifiers.

### Added
- **`cha_core::query`** — host-side tree-sitter query helper (`run_query` / `run_queries` / `node_to_match`). Both built-in plugins and the WASM `tree_query` host import now go through this single API.
- **`DeadCodeAnalyzer::entry_points`** — entry-point names are now configurable via `[plugins.dead_code] entry_points = [...]`. Default list expanded from Rust-only (5 names) to multi-language (Rust + Python `__init__` etc + Go `init` + C `_start` + tokio).
- **`LengthAnalyzer::complexity_factor_threshold`** — was hardcoded `10.0`, now configurable via `[plugins.length]`.

### Fixed
- **`unsafe_api`**: rewritten from line-based `line.contains` + odd-quote-count heuristic to per-language tree-sitter queries. Picks up real `sprintf`/`strcpy`/`strcat`/`system` call sites that the line-based heuristic missed. Comments and string literals containing keywords like `unsafe` no longer false-positive.
- **`dead_code`**: substring `is_in_file_referenced` replaced with AST identifier scan. Token-concat macro detection rewritten — instead of nuking the entire file when any `#define ... ##` exists, parse define bodies for `prefix##X##suffix` slots, scan call sites for invocation arguments, synthesize plausible expansion names, and add them to the reference set. X-macro dispatch tables (e.g. `STYLE_DEF`) no longer hide every dispatch function. `IdentifierPositions` lookup is now O(1) per symbol via `HashMap<name, Vec<line>>`.
- **`error_handling`**: `unwrap_abuse` uses tree-sitter (`(call_expression field_expression unwrap|expect)`); empty-catch detection is per-language (Rust skipped, TS `catch_clause`, Python `except_clause`). String literals and comments containing the substring `unwrap` or `catch` no longer trigger.
- **`hardcoded_secret`**: regex matches now run against `string_literal` node text only, not full source lines. Comments and identifier names with secret-like substrings no longer false-positive.
- **`cha fix`**: `String::replace` whole-content substitution replaced with tree-sitter identifier-node range collection + byte-offset reverse substitution. The previous implementation could rewrite identifier names inside string literals and comments, corrupting source files.
- **`git_metrics::check_test_ratio`**: `f.contains("test") || f.contains("spec")` replaced with `cha_core::is_test_path`. The substring check wrongly counted `request.rs` / `spectrum.rs` etc. as test files, polluting the test-to-production ratio that drives `low_test_ratio`.
- **`wasm.rs::infer_file_role`**: replaced duplicate test-path heuristics with `cha_core::is_test_path`. WASM plugins' `FileRole::Test` classification now matches the canonical convention used elsewhere (`__tests__/`, `__mocks__/`, `.test.ts`, `.spec.ts`).
- **`find_macro_invocation_args`**: word-boundary check added — `STYLE_DEF` no longer matches `STYLE_DEFINE` invocations.

### Removed
- **`unsafe_api` `is_in_string` heuristic** — superseded by tree-sitter queries that distinguish string literals at the AST level.
- **`error_handling` line-based `detect_empty_catch`** — replaced with grammar-aware queries.
- **`HostState::query_cache`** — query compilation now lives in `cha_core::query` (compile-on-demand; LRU caching to be added if measurement warrants).

## [1.17.0] - 2026-05-21

### Added
- **`project_query::function_at(path, line, col)`** — new host import returning the `FunctionInfo` whose body contains the given position. Useful for tree-query–driven detectors that need to disambiguate which declared function a queried position belongs to.
- **`WasmPluginTest::option_list / option_bool / option_int / option_float`** — list and typed option setters in the test harness, replacing the previous string-only `option()`.

### Changed (breaking for WASM plugins)
- **`tree_query::QueryMatch.start_line` / `end_line` are now 1-based** (was 0-based). Aligns with `FunctionInfo` / `ClassInfo` / `CommentInfo` line numbering — no more per-plugin off-by-one conversion. Inputs to `node_at(line, col)` and `nodes_in_range(start, end)` are likewise 1-based now.
- Existing plugins compiled against the pre-1.17 WIT will need to be rebuilt against the new SDK; instantiation will fail loudly otherwise.

### Fixed
- **`react-hooks` example plugin** — false positives on `hook_after_early_return` (in sibling components and inside return expressions like `return useState()`) eliminated by switching to `project_query::function_at` for host-function disambiguation. Now reports 5 true positives / 0 false positives on the 6-component .tsx fixture (was 5 / 2).

### Documentation
- `docs/plugin-development.md`: added Line/Column convention note, Project Query API section, WASM Compatibility Cheatsheet (regex panics, no clock, no FS), `cha plugin build` vs `cargo build` distinction, and new option helpers in Testing.

## [1.16.0] - 2026-05-21

### Added
- **`TsxParser`** in `cha-parser` — `.tsx` files now route to a parser using `tree_sitter_typescript::LANGUAGE_TSX`, so JSX nodes (`jsx_element`, `jsx_attribute`, `jsx_self_closing_element`) are first-class AST citizens. WASM plugins can now match them via `tree_query::run_query`.
- **`examples/wasm-plugin-react-hooks`** — example WASM plugin demonstrating `tree_query` integration. Detects 5 React Rules of Hooks violations: hooks called from non-component functions, hooks in conditionals, hooks in loops, hooks after early return, and hooks in nested callbacks.
- **`examples/wasm-plugin-todo-tracker`** — example WASM plugin demonstrating extended TODO comment tracking beyond the builtin `todo_tracker`. Adds 5 new smells: extended tag set (BUG/WIP/OPTIMIZE/PERF/DEPRECATED + user-configurable extras), `(by:YYYY-MM-DD)` expiration, priority escalation (`!`/`!!`/`!!!`), per-file TODO hotspot detection, and required-attribution policy.

### Notes
- WIT unchanged at `cha:plugin@0.3.0` (no breaking change).
- Routing for `.ts` / `.mts` / `.cts` continues to use `LANGUAGE_TYPESCRIPT`. Only `.tsx` switched.

## [1.15.0] - 2026-05-14

### Added
- **`ProjectQuery` trait** in `cha-core` — plugins now access cross-file data through a typed interface on `AnalysisContext.project` instead of host-side post-hoc string-matched filtering. 12 methods cover the project-level queries existing post-analysis passes need: `is_called_externally`, `callers_of`, `function_home`/`function_by_name`/`class_home`, `is_third_party`, `workspace_crate_names`, `is_test_path`, etc. WASM plugins also gain access via the `project-query` host import.
- **`ProjectQueryBulk` trait** extends `ProjectQuery` for in-process iteration (`iter_models`); not exposed to WASM.
- **`cha_core::is_test_path`** — public utility consolidating two duplicated implementations.
- **example-wasm `unused_helper` smell** — demonstrates `project_query::callers_of` callback.

### Changed
- **WIT bumped to `cha:plugin@0.3.0`** (breaking) — adds `project-query` host import. External plugins compiled against `0.2.0` must rebuild.
- **`large_api_surface` C/C++ heuristics** — `.h/.hpp` headers are now skipped (their 100% public surface is by design); `.c/.cpp` implementation files use a higher count threshold (30, configurable as `c_max_exported_count`) and the ratio gate is effectively off (configurable as `c_max_exported_ratio`). lvgl baseline: 393 → 34 findings (-91%).
- **`dead_code` is now project-aware** — uses `ProjectQuery::is_called_externally` to confirm cross-file usage; the per-file text search is just an early shortcut. The token-concat macro heuristic (`#define ... ##`) remains because parsers don't macro-expand. lvgl baseline: 67 → 6 findings (-91%).

### Removed
- **`Plugin::cross_file_aware_smells`** trait method — replaced by typed query through `AnalysisContext.project`.
- **`cha-cli::cross_file_filter` module** — the post-hoc string-matched filter is gone; plugins produce final findings using the typed trait.
- **3 duplicated `workspace_crate_names` impls** + 3 duplicated `is_third_party`/`is_external_leak` impls + 2 duplicated `is_test_path` impls — all consolidated.

### Added
- New `.cha.toml` config keys for `api_surface`: `max_exported_ratio`, `c_max_exported_count`, `c_max_exported_ratio`, `skip_c_headers`. All language-aware defaults preserved.

## [1.14.0] - 2026-05-14

### Added
- **Plugin AST Query API** — WASM plugins can now execute tree-sitter queries against the current file's AST via the `tree-query` host import interface (`run-query`, `run-queries`, `node-at`, `nodes-in-range`). Enables plugins to do custom structural pattern matching without reimplementing parsing.
- **`file-role` enum** in `analysis-input` — host infers whether a file is `source`, `test`, `doc`, `config`, or `generated` from its path, allowing plugins to apply differential detection strategies.
- **SourceModel enrichment** — `analysis-input` now includes `comments`, `type-aliases`, `parameter-names`, `switch-arm-values`, and `is-module-decl` fields previously only available to internal plugins.
- **`parse_file_full()`** in `cha-parser` — returns `ParseResult` carrying model + tree-sitter `Tree` + `Language` for downstream use by WASM host callbacks.

### Changed
- **WIT bumped to `cha:plugin@0.2.0`** — breaking change: plugins compiled against `0.1.0` must be recompiled. No behavioral change for existing internal plugins.

## [1.13.1] - 2026-04-30

### Added
- **`abstraction_leak_surgery` detector** — files that co-change in git history **and** share a third-party type in their function signatures. Upgrade of the classic `shotgun_surgery`: instead of "these files always change together" (agnostic of why), this pinpoints "these files always change together *because* they all depend on the same external type" — the shared external type is the concrete abstraction leak driving the co-change cascade. Severity `Hint`.
  - Inputs: git co-change counts (`git log --name-only -N`, threshold ≥ 5 commits in last 100) × per-file `TypeOrigin::External` sets derived from parameter / return types. Workspace-sibling crates auto-whitelisted (same mechanism `cross_boundary_chain` / `leaky_public_signature` use), so `cha_core`-internal dependencies between `cha-parser` / `cha-cli` don't fire.
  - Cha self-baseline: 10 genuine findings, all pointing at the 5 language parsers sharing `tree_sitter::Node` — exactly the abstraction leak the detector is designed to find (tree-sitter upgrades ripple across every parser file). lvgl `src/`: 0 (C project, no External origins).

## [1.13.0] - 2026-04-30

### Added
- **`primitive_representation` detector** (roadmap S8.2). Flags function parameters whose **name** carries a domain concept (`user_id`, `email`, `status_code`, `api_url`, `password`, `language`, …) but whose **type** is a raw scalar primitive (`String`, `i32`, `bool`, `char`, …). Signals an opportunity to introduce a newtype / value object to preserve the invariant. Per-parameter detection groups all offending params of one function into a single hint. Complements the existing `primitive_obsession` (which looks at per-function ratio): this fires on even a single param when it's clearly a business concept.
  - Business-token and noise-token vocabularies are deliberately narrow to keep signal-to-noise high. Substring matches are ruled out (tokens must be standalone words — `widget_identifier` does not trigger on `id`).
  - Parameters already typed with project-local newtypes (e.g. `id: UserId` where `UserId` is `TypeOrigin::Local`) are skipped — the author already did the right thing.
  - Container types (`Path`, `PathBuf`, `Vec`, `Arc`, `Box`, `HashMap`, …) are treated as domain-carrying and excluded; wrapping `path: &Path` in a newtype would destroy the abstraction.
  - Only runs on `is_exported` functions — private helpers are noise for a design signal aimed at public API hygiene.
  - Cha self-analyze: 14 findings (all genuine — `rel_path`/`env_hash`/`language`/`key`/`hash` as raw types). lvgl `src/` baseline: 53 findings (TTF `platformID/encodingID/languageID/nameID: int`, file-explorer `path/dir: char` pointers, …).
- **`stringly_typed_dispatch` detector** (roadmap S8.8). Flags functions whose `switch`/`match` body dispatches on ≥ 3 **string** or ≥ 3 **integer** literal arms — classic "the arm values should have been an enum" smell. Char-literal arms (C tokenisers) skipped. Enum-variant / structural-pattern arms classify as `Other` and never contribute to the threshold, so `match event { Event::Click => …, Event::Scroll => …, _ => … }` stays quiet while `match s { "click" => …, "scroll" => …, "submit" => … }` fires. Severity `Hint`. Complements S8.2 `primitive_representation` (signature side) with the body-side dispatch signal.
  - New `cha_core::ArmValue` enum (`Str / Int / Char / Other`) + `FunctionInfo.switch_arm_values` + `FunctionSymbol.switch_arm_values`. Populated by every parser via a new shared `cha-parser/src/switch_arms.rs` helper — language-specific arm-node kinds funnel through one classifier.
  - Cha self-baseline: 20 findings (all node-kind dispatchers in the 6 language parsers — valid detections, users can add `// cha:ignore stringly_typed_dispatch` if the dispatch shape is forced by tree-sitter). lvgl `src/` baseline: 23 findings (PNG/JPEG/QR error-code dispatchers, color-format size tables, TTF bytecode interpreter).
- **`cross_boundary_chain` detector** (roadmap S8.U4). Flags functions where `chain_depth ≥ 3` **and** the chain's root parameter is externally-typed (`TypeOrigin::External(crate)`) — the function is reaching into a third-party library's internal field layout, not just over-chaining local data. Companion to the existing `message_chain` (which fires on depth regardless of source): `cross_boundary_chain` is narrower but a stronger abstraction-leak signal. Severity `Hint`.
  - Workspace crates are auto-whitelisted (same mechanism `leaky_public_signature` uses), so sibling `cha_core::Finding` traversals inside this repo don't fire. Cha self-baseline: 4 findings, all genuine `tree_sitter::Node` traversals in `cha-parser`. lvgl `src/` baseline: 0 (C project, few `External` origins by design).
  - Zero parser changes — reuses `chain_depth`, `parameter_types` (with origin), `parameter_names`, `external_refs`. Pure post-pass on `ProjectIndex`.
- **`FunctionInfo.parameter_names` + `FunctionSymbol.parameter_names`** (`cha-core`). Parallel to `parameter_types`: identifier names in declaration order. All six parsers (Rust / TS / Python / Go / C / C++) extract these; `self` / C++ `this` positions skipped to stay length-aligned with `parameter_types`. Enables name-semantic analyses like `primitive_representation`, future LSP hover with full signatures, future `cha summary`.
- New helpers `cha_parser::rust_imports::rust_param_names` and `cha_parser::cpp::c_param_name` extract identifier names from their language's declarator chains; reused across all C/C++ function-definition sites.

## [1.12.0] - 2026-04-28

### Added
- **`SymbolIndex` — structural view of a file, cached separately from `SourceModel`.** New type in `cha-core::model` carrying the fields consumers like `cha deps`, LSP workspace-symbols, and future `cha summary` all share — class/function names + signatures + positions + `type_aliases` — without per-function-body data (complexity, body hash, TypeRef origin, cognitive, chain depth etc. stay in `SourceModel`).
  - `ProjectCache::{get,put}_symbols` store to `symbols/{chash}.bin`, mirrored independently of `parse/{chash}.bin`. Same `env_hash` mechanism invalidates both on parser code changes.
  - `cached_symbols(path)` is a new warm fast path that skips `SourceModel` deserialisation entirely — `symbols/{chash}.bin` is roughly 10% the size of `parse/{chash}.bin`.
  - `cached_parse` now populates both caches on every fresh parse, so the two views are always in lockstep.
  - `lvgl src/` warm benchmarks (379 files): `deps --type imports` 1.28s → 38ms (34×), `--type classes` 1.30s → 56ms (23×), `--type calls` 1.30s → 48ms (27×). Edge counts unchanged vs. pre-migration (1351/142/8109).
  - `cha-cli/src/c_oop_enrich` grows a `enrich_c_oop_symbols` / `attribute_methods_by_name_from_symbols` pair alongside the existing `SourceModel` functions. Shared `attribute_one_raw` keeps attribution rules single-sourced; build-index / write-back are deliberate parallel code paths because the two storage types have to stay independent.
  - `cha-cli/src/parse_cache.rs` (new module) hosts both `cached_parse` and `cached_symbols`.
- **C++ parser now handles `ClassName::method()` out-of-class definitions, namespaces, and templates.** Three gaps in the previous CppParser have been closed:
  - `void Foo::bar() {...}` (and `::global()`, `A::B::c()`, destructors `Foo::~Foo()`, operators `Foo::operator+()`) was silently dropped — `find_func_name_node` only accepted bare `identifier` declarators. It now also unwraps `qualified_identifier`, `destructor_name`, and `operator_name`.
  - Out-of-class method definitions now attribute to their owning same-file class: `void Foo::bar()` bumps `ClassInfo::method_count` on `Foo` and flips `has_behavior`. Cross-file attribution still runs through `cha-cli::c_oop_enrich`.
  - `namespace_definition`, `linkage_specification` (`extern "C" { ... }`), and `template_declaration` are now explicitly matched in the top-level dispatch (previously fell through to the generic recursion arm) — same observable behaviour, but the nesting constructs are now a stable hook rather than an accidental default-case artefact.
  - C++-specific declarator helpers moved to a new `cha-parser/src/cpp.rs` so `c_lang.rs` stays below the `large_file` gate.
- **`SourceModel.type_aliases` now populated for Rust, TypeScript, Python, and Go** (previously all four returned empty `vec![]` with parser-side TODOs). Each parser recognises its language's alias form and records `(alias, rhs)` pairs: Rust `type X = Y;` / `pub type X<T> = Y;`, TypeScript `type X = Y;` / `export type X<T> = Y;`, Python 3.12+ `type X = Y` and pre-3.12 `X: TypeAlias = Y`, Go `type X = Y` (only the true alias form — `type X Y` defined types are excluded). Plain Python `X = Y` assignments remain unclassified (too ambiguous). Shared extraction lives in a new `cha-parser/src/type_aliases.rs` module so per-language files stay below the `large_file` gate.

### Changed
- **`boundary_leak` detector migrated to `ProjectIndex`.** The three smells it emits (`abstraction_boundary_leak`, `return_type_leak`, `test_only_type_in_production`) previously parsed the whole project a second time — the codebase noted a "cached model occasionally drops typedef aliases" concern with root cause TBD. v1.11.0's binary-mtime cache keying removed the suspected root cause, and a new `cache::tests::cache_roundtrip_preserves_type_aliases` unit test makes the invariant a testable one. `boundary_leak::detect` now takes `&ProjectIndex` and shares the same parse pass as `anemic_domain_model`, `typed_intimacy`, `module_envy`, and friends. Verified against lvgl's `src/` tree: 155 findings before = 155 findings after (`abstraction_boundary_leak: 154, return_type_leak: 1`). Completes roadmap S8.infra.4.

### Fixed
- **C++: template specialisation methods attribute to the right class.** `template<> void Foo<int>::bar()` used to drop on the floor because the qualifier `Foo<int>` (a `template_type` node) didn't match the stored class name `Foo`. `attach_to_class` now strips trailing `<...>` template arguments before matching, so out-of-class specialisations attribute correctly. Same stripping applies to any declaration whose declarator surfaces `Foo<...>` as the owning scope.
- **C++: real inheritance (`class Derived : public Base`) now recognised.** `extract_class` consults the `base_class_clause` child and pulls the first `type_identifier` (or `template_type`'s underlying name) as `parent_name`. Falls back to the first-field heuristic only when no base clause is present, so legacy C struct-embedding cases still work. Also fixes the class-name extraction for templated classes so `template<typename T> class Foo {...}` stores `"Foo"` instead of `"Foo<T>"`.
- **C++: reference-return methods no longer vanish.** `const int& Foo::get()` and similar `reference_declarator`-wrapped definitions used to be silently dropped because tree-sitter-cpp's `reference_declarator` has no `declarator` field — the declarator walker returned `None`. Both `find_func_name_node` (c_lang) and `descend_to_qualified_identifier` (cpp) now fall back to the first named child when the field is absent. Same fix path covers reference-return + qualified (`const T& Foo::bar()`) so class attribution still works. 6 additional regression tests (reference/pointer return types, multi-method attribution, constructor, `extern "C"`, const member) added in `cha-parser/tests/cpp_enhancements.rs` (14 total).

## [1.11.1] - 2026-04-27

### Changed
- Internal: split git-backed post-analysis passes (`unstable_dependency`, `bus_factor`, `low_test_ratio`) out of `cha-cli/src/analyze.rs` into a new `cha-cli/src/git_metrics` module. No behaviour change; `analyze.rs` drops below the 850-line `large_file` threshold that `cargo xtask analyze` gates on. `collect_top_level` in the C parser also picks up `// cha:ignore high_complexity` alongside the existing cognitive-complexity ignore after the `declaration` arm added one branch.

Note: 1.11.0 was tagged in the repo but the CI self-analyze gate failed on the above source-dir warnings so crates.io was never updated. 1.11.1 is the first shipped release of the 1.11 line.

## [1.11.0] - 2026-04-27

### Fixed
- **Cache invalidation now tracks the cha binary**, not `CARGO_PKG_VERSION`. `env_hash` folds in `std::env::current_exe()`'s mtime, so any new binary — developer rebuild after editing parser code, or end-user upgrade to a new release — invalidates stale cached `SourceModel` entries. The previous version-based key allowed parser behaviour changes shipped without a `cargo xtask bump` to silently serve wrong cached data (which is what hid the header-declaration parser fix from users with existing `.cha/cache`). Falls back to `CARGO_PKG_VERSION` when `current_exe()` fails (unusual — sandboxed runners).
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
