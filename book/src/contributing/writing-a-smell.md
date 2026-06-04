# Writing a smell

Three files, four steps. We'll walk through `MiddleManAnalyzer` ([source](https://github.com/W-Mai/Cha/blob/main/cha-core/src/plugins/middle_man.rs)) — 64 lines — as the worked example.

This page is for **built-in** smells (the ones that ship in `cha-core`). For project-specific detectors that don't belong in the main repo, write a WASM plugin instead — see [Custom plugin in 50 lines](../recipes/custom-plugin-50loc.md).

## Step 1: write the analyzer

Create `cha-core/src/plugins/<your_smell>.rs`:

```rust
use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

pub struct MiddleManAnalyzer {
    pub min_methods: usize,
    pub delegation_ratio: f64,
}

impl Default for MiddleManAnalyzer {
    fn default() -> Self {
        Self {
            min_methods: 3,
            delegation_ratio: 0.5,
        }
    }
}

impl Plugin for MiddleManAnalyzer {
    fn name(&self) -> &str { "middle_man" }
    fn smells(&self) -> Vec<String> { vec!["middle_man".into()] }
    fn description(&self) -> &str { "Class that only delegates to others" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model.classes.iter()
            .filter(|c| {
                c.method_count >= self.min_methods
                    && c.delegating_method_count > 0
                    && (c.delegating_method_count as f64 / c.method_count as f64)
                        >= self.delegation_ratio
            })
            .map(|c| Finding {
                smell_name: "middle_man".into(),
                category: SmellCategory::Couplers,
                severity: Severity::Hint,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: c.start_line,
                    start_col: c.name_col,
                    end_line: c.start_line,
                    end_col: c.name_end_col,
                    name: Some(c.name.clone()),
                },
                message: format!(
                    "Class `{}` delegates {}/{} methods, acting as a middle man",
                    c.name, c.delegating_method_count, c.method_count
                ),
                suggested_refactorings: vec!["Remove Middle Man".into()],
                actual_value: Some(c.delegating_method_count as f64 / c.method_count as f64),
                threshold: Some(self.delegation_ratio),
                risk_score: None,
            })
            .collect()
    }
}
```

Conventions:

- **Struct fields are the thresholds.** No magic numbers in `analyze()`. Defaults go in `Default`.
- **`name()` is the plugin handle.** Used in `--plugin <name>`, `[plugins.<name>]` config, `// cha:ignore <name>`.
- **`smells()` lists every smell name the plugin emits.** Most plugins emit one smell that matches `name()`; some plugins emit multiple (e.g. `length` emits `long_method`, `large_class`, `large_file`).
- **`Severity::Hint` for stylistic findings, `Warning` for things that meaningfully harm readability/correctness, `Error` for things CI should refuse.**
- **`actual_value` and `threshold` carry the numbers** that show up in messages and the `--explain` output. Always populate them when there's a numeric metric.

## Step 2: pick a `SmellCategory`

The category drives grouping in CLI output, JSON reports, and `--focus`. Match what the smell actually is:

| Category | What lives here |
|---|---|
| `Bloaters` | Code that has grown too large (`long_method`, `god_class`, `complexity`). |
| `Couplers` | Modules that depend too tightly on each other (`coupling`, `feature_envy`, `middle_man`). |
| `OOAbusers` | Object-oriented constructs used incorrectly (`switch_statement`, `refused_bequest`, `design_pattern`). |
| `ChangePreventers` | Change in one place forces changes elsewhere (`shotgun_surgery`, `divergent_change`). |
| `Dispensables` | Code that can be removed without losing function (`dead_code`, `duplicate_code`, `lazy_class`). |
| `Security` | Risky calls and leaked secrets (`hardcoded_secret`, `unsafe_api`). |

If your smell straddles two — pick the more specific one. Categories don't compose.

## Step 3: register it

Edit [`cha-core/src/plugins/mod.rs`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/plugins/mod.rs):

```rust
mod middle_man;
pub use middle_man::MiddleManAnalyzer;
```

Edit [`cha-core/src/registry.rs`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/registry.rs). Find the appropriate `register_*_plugins` function for your category and add:

```rust
register_if_enabled(plugins, config, "middle_man", || {
    let mut p = MiddleManAnalyzer::default();
    apply_usize(config, "middle_man", "min_methods", &mut p.min_methods);
    apply_f64(config, "middle_man", "delegation_ratio", &mut p.delegation_ratio);
    Box::new(p)
});
```

`apply_*` reads `[plugins.middle_man]` from `.cha.toml` and overrides the default thresholds. Skip the `apply_*` calls if your analyzer has no configurable fields.

`register_if_enabled` honours `enabled = false` in `[plugins.middle_man]` — you don't need to handle that case yourself.

## Step 4: tests + docs

Create `cha-core/src/plugins/<your_smell>_tests.rs` (or add to an existing test file). Pattern:

```rust
#[test]
fn fires_on_middle_man() {
    let src = r#"
        class Wrapper {
            fn foo(&self) { self.inner.foo() }
            fn bar(&self) { self.inner.bar() }
            fn baz(&self) { self.inner.baz() }
        }
    "#;
    let findings = analyze_with(MiddleManAnalyzer::default(), "rust", src);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "middle_man");
}

#[test]
fn does_not_fire_below_threshold() {
    // ... 2 delegating methods, default min_methods=3
}
```

Keep tests narrow: one fires-when-expected, one ignores-when-below-threshold, one for each interesting edge case. The fixture-based tests (under `cha-core/tests/fixtures/`) are for cross-plugin behaviour; for unit testing a single plugin, inline source strings are clearer.

Then update three docs:

1. **README.md plugin table** — add a row to the appropriate `SmellCategory` section, with smell name, default thresholds, severity. Plus the same row in **README.zh-CN.md**.
2. **docs/plugins.md** — full description with a "what triggers it" example. Plus **docs/plugins.zh-CN.md**.
3. **CHANGELOG.md** under `[Unreleased]` — one line under "Added".

The book's plugin reference page is generated from `docs/plugins.md` via `{{#include}}`, so you don't edit it directly.

## Verify

```bash
cargo xtask ci   # runs build + test + lint + analyze
```

Then dogfood — run the new plugin against the Cha codebase itself:

```bash
cargo run -- analyze --plugin middle_man cha-core/
```

If it reports findings on Cha's own code, decide: are they real (fix Cha) or false positives (tighten the analyzer)?

## See also

- [`Plugin` trait](https://github.com/W-Mai/Cha/blob/main/cha-core/src/plugin.rs)
- [`SmellCategory` enum](https://github.com/W-Mai/Cha/blob/main/cha-core/src/model.rs)
- [Architecture](./architecture.md) — how the plugin fits into the data flow.
- [Plugin development](../plugins/development.md) — for WASM plugins (out-of-tree).
