# FAQ

### How does Cha differ from clippy?

Clippy is a Rust-only linter focused on idiom and correctness ("you wrote `x.iter().count()`, use `x.len()`"). Cha is a multi-language code-smell tool focused on design ("this function is 200 lines and reaches into 9 other types"). The two don't overlap and run fine alongside each other. See [Migrate from clippy](./recipes/migrate-clippy.md) for the boundary.

### Why detect smells instead of lints?

Lints catch local bugs — a misuse the compiler could almost have caught. Smells catch design problems — code that compiles, runs, and passes tests, but will be expensive to change. The two need different thresholds, different severities, and different remediation advice, so Cha treats them as a separate tool rather than tacking design rules onto an existing linter.

### Why is my test or generated file being flagged?

Add it to `exclude` in `.cha.toml`:

```toml
exclude = ["*/tests/fixtures/*", "vendor/*", "**/*.generated.rs"]
```

For one-off cases inside a real source file, use an inline directive:

```rust
// cha:ignore                  — suppress all rules for the next item
// cha:ignore long_method      — suppress one rule
```

See [Inline directives](./configuration/inline-directives.md).

### Should I baseline existing issues or fix them?

Baseline for legacy code — run `cha baseline` once, commit `.cha/baseline.json`, and CI only fails on new findings. Fix for code you're actively working on; the baseline file is a freeze, not a forever-exemption. Workflow detail: [Suppressing legacy issues](./recipes/suppress-legacy.md).

### How does the strictness scale work?

`strictness` in `.cha.toml` is a multiplier applied to every threshold: `relaxed = 2.0×`, `default = 1.0×`, `strict = 0.5×`. So `strictness = "strict"` halves `max_function_lines`, `warn_threshold`, `max_imports`, and so on. Custom floats (e.g. `0.7`) are accepted. Per-plugin and per-item overrides win over the global multiplier.

### Can I disable a single smell?

Project-wide, in `.cha.toml`:

```toml
[plugins.message_chain]
enabled = false
```

Or per-item via `// cha:ignore message_chain` on the line above the offending item.

### Does Cha modify my code?

No, except when you explicitly run `cha fix --apply`. Today `fix` only handles naming-convention rewrites (e.g. PascalCase for types); everything else is read-only. `cha analyze`, the LSP, and CI runs never write to source files. See [`cha fix`](./cli/fix.md).

### How do I write a custom plugin?

Cha plugins are WebAssembly Component Model modules — any language that compiles to WASM Components can write one. Scaffold with `cha plugin new <name>`, build with `cha plugin build`, install with `cha plugin install`. End-to-end walkthrough and SDK reference: [Plugin development](./plugins/development.md).

### Why doesn't the LSP support completion or rename?

Cha's LSP is a diagnostics-and-insight server, not a language server in the Rust-Analyzer sense. It owns the smell-detection capabilities (diagnostics, CodeLens, hover, inlay hints, workspace scan) and stays out of completion/rename/goto-def, which your existing language server already handles. Run Cha alongside `rust-analyzer` / `pyright` / `gopls` rather than instead of them. Capability list: [LSP overview](./lsp/overview.md).

### How is performance on large repos?

Cha uses a two-level cache (L1 in-memory + L2 bincode on disk) keyed by file mtime, so unchanged files skip parsing entirely. On the 3,201-file NuttX RTOS tree, warm-cache `analyze` runs in 3.3s vs. 5.7s cold (~26× speedup over the slow paths). For incremental flows, prefer `cha analyze --diff` so only changed files are inspected.

### How do I upgrade?

Re-run the installer you originally used:

```bash
# Shell installer
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.sh | sh

# Homebrew
brew upgrade cha-cli
```

For the GitHub Action, bump the `rev:` / `uses:` tag in your workflow / pre-commit config (e.g. `W-Mai/Cha@v1.20.0`).

### Which language has the best coverage?

Rust and TypeScript are the most mature — every detector that depends on AST features (classes, traits, generics, exception flow) works on both. Go and Python have full support across all detectors that don't require trait/interface introspection. C and C++ run all the structural detectors (length, complexity, dead code, coupling, hotspot, layers); detectors that depend on OO constructs (`naming`, `lazy_class`, `data_class`, `design_pattern`) are disabled by the C language preset.
