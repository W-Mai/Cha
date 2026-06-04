# Migrate from clippy

`clippy` and `cha` answer different questions. Clippy is the Rust language linter — it knows about borrow checks, idioms, lifetime hazards. Cha looks across files for *design* problems — long methods, god classes, feature envy, hub-like dependencies, layer violations.

You don't replace clippy with Cha. You run both. This page covers the two friction points that come up.

## 1. Run them side by side

Cha never touches `cargo clippy` output. The two tools share neither config nor lockfile. Add Cha to whatever you already do:

```bash
# Existing
cargo clippy --all-targets -- -D warnings

# New
cha analyze --fail-on warning
```

In CI, run them as two steps. If clippy fails on a borrow-check rule, Cha shouldn't run; if Cha fails on a design rule, clippy already passed.

## 2. Tune Rust thresholds

Cha defaults are language-agnostic. Two thresholds tend to need bumping for typical Rust code:

```toml
# .cha.toml
[plugins.length]
max_function_lines = 60   # Rust signatures + match arms eat lines fast

[plugins.complexity]
warn_threshold = 12
error_threshold = 24      # match-heavy code legitimately runs higher than 10
```

Run `cha calibrate` first to see what your project's P90 / P95 actually are; then decide whether to use those numbers or stay closer to defaults. See [Calibrate to your codebase](./calibrate.md).

## 3. Map clippy lints to Cha smells (where they overlap)

Most clippy lints don't have a Cha analogue and most Cha smells don't have a clippy analogue. The small overlap:

| clippy lint | Cha smell | Notes |
|---|---|---|
| `too_many_arguments` | `long_parameter_list` | Clippy: 7 by default. Cha: 5. |
| `cognitive_complexity` | `cognitive_complexity` | Same metric (SonarSource), independent thresholds. |
| `large_stack_arrays` | — | Stack-size analysis is out of scope for Cha. |
| `mod_module_files` | — | Style; not a Cha concern. |

Where the lint exists in both, you usually want to keep clippy's check on (it sees the AST at type level) and let Cha look across functions.

## 4. Suppress noise from auto-generated code

If clippy's `#[allow(...)]` already covers a generated file, Cha respects nothing automatically. Either add the path to `exclude`:

```toml
exclude = ["src/generated/**", "build/**"]
```

…or use an inline directive at the top of the offending item:

```rust
// cha:ignore
fn handler_generated_by_macro() { /* ... */ }
```

See [Inline directives](../configuration/inline-directives.md).

## See also

- [Configuration overview](../configuration/overview.md)
- [Calibrate to your codebase](./calibrate.md)
- [Suppress in legacy code](./suppress-legacy.md)
