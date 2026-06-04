# Suppress in legacy code

You're adopting Cha mid-flight. The first `cha analyze` reports 200 findings, half of them in code older than the team currently working on the project. Make CI green without lying about the codebase.

Three tools, in priority order: **baseline** for "we'll come back to this", **inline directives** for "this specific item is fine", **config exclude** for "Cha shouldn't look at all".

## 1. Baseline first

Generate a snapshot of every existing finding. Future runs only fail on new ones.

```bash
cha baseline
git add .cha/baseline.json && git commit -m "Cha baseline at adoption"
```

CI uses it:

```bash
cha analyze --baseline .cha/baseline.json --fail-on warning
```

Now PRs that introduce new findings fail. Old findings are silent. The baseline file is small (a list of fingerprints), commits cleanly, and diffs readably when entries get removed as debt is paid.

Full workflow: [Baseline workflow](./baseline.md).

## 2. Inline directives for specific items

When a single item legitimately breaks a rule (a 200-line state machine, a 9-parameter constructor that can't be helped):

```rust
// cha:ignore long_method
fn dispatch_state_machine(&mut self, event: Event) -> State {
    match self.current {
        // ... 200 legitimate lines
    }
}
```

```python
# cha:ignore long_parameter_list
def __init__(self, host, port, user, password, db, ssl_cert, retry, timeout):
    ...
```

Suppress one rule, multiple, or everything for the next item:

```rust
// cha:ignore                        — suppress all
// cha:ignore long_method            — suppress one
// cha:ignore long_method,complexity — suppress multiple
// cha:set long_method=200           — bump the threshold for this item only
```

Inline directives **do not appear in baseline files** — they're explicit decisions in source. Use them when you want the suppression visible in code review.

See [Inline directives](../configuration/inline-directives.md) for the full grammar.

## 3. Config exclude for whole paths

Some files Cha shouldn't see at all — generated code, third-party vendoring, fixture files for tests:

```toml
# .cha.toml
exclude = [
    "vendor/**",
    "src/generated/**",
    "tests/fixtures/**",
    "node_modules/**",   # tree-walker honours .gitignore, so this is usually unnecessary
]
```

Patterns are globs. `**` matches any depth. Excluded paths are not parsed at all — cheaper than running and suppressing.

## Decision matrix

| Situation | Tool |
|---|---|
| Existing findings everywhere; need green CI today | Baseline |
| One file with one stubborn finding | Inline `cha:ignore` |
| One file with a unique threshold need | Inline `cha:set` |
| Whole directory shouldn't be analyzed | Config `exclude` |

Combine freely. Baseline + inline + exclude are independent layers; the order Cha applies them is `exclude` → analyze → `cha:ignore`/`cha:set` → baseline filter. A finding survives only if all four let it through.

## Paying down debt

The baseline is not "ignore forever". Periodically:

```bash
cha baseline                       # regenerate, captures new state
git diff .cha/baseline.json        # see what shrank
```

If `git diff` shows entries removed, debt was paid. If entries appeared, you've added findings on top of the snapshot — investigate whether `--baseline` was being respected in CI.

## See also

- [Baseline workflow](./baseline.md)
- [Inline directives](../configuration/inline-directives.md)
- [`cha baseline`](../cli/baseline.md)
