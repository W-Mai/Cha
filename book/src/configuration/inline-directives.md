# Inline directives

Suppress findings or relax thresholds for a single function or class by adding a comment immediately above (or on the same line as) the item.

## `cha:ignore` — suppress findings

| Form | Effect |
|------|--------|
| `// cha:ignore` | Suppress all rules for the next item. |
| `// cha:ignore <name>` | Suppress one rule by smell name. |
| `// cha:ignore <a>,<b>` | Suppress multiple rules (comma-separated). |

The `<name>` is the *smell* name as it appears in CLI output (e.g. `long_method`, `high_complexity`, `switch_statement`), not the plugin name. A plugin like `length` emits three different smells, so be specific.

## `cha:set` — override thresholds

| Form | Effect |
|------|--------|
| `// cha:set <smell>=<n>` | Raise the threshold for one smell on this item. |
| `// cha:set threshold=<n>` | Raise the threshold for every threshold-based rule on this item. |

`<n>` is parsed as a float. If the actual measured value is still above the new threshold, the finding is kept.

`cha:set` only affects findings that report numeric `actual_value` and `threshold` fields. Boolean detectors (e.g. `inappropriate_intimacy`) ignore `cha:set` — use `cha:ignore` for those.

## Comment styles

All directives work with `//` (Rust, TypeScript, Go, C, C++), `#` (Python), `--` (Lua, SQL), and `/* … */` block comments:

```rust
// cha:ignore long_method
```

```python
# cha:ignore long_method
```

```c
/* cha:ignore long_method */
```

The directive must be the *first thing* on the line (after stripping whitespace and the comment marker). Trailing comments after code are not parsed.

## Coverage rules

A directive applies to a finding when either:

1. The directive is on the same line as the finding's start, **or**
2. The directive is at most 2 lines above the finding's start.

This lets you stack multiple directives:

```rust
// cha:ignore long_method
// cha:set high_complexity=25
fn complicated_but_acknowledged() {
    // …
}
```

Directives further than 2 lines above the item have no effect.

## Examples

### Silence one rule on a function

```rust
// cha:ignore long_method
fn render_template(/* … */) -> String {
    // 200-line template builder; we know.
}
```

### Silence multiple rules

```typescript
// cha:ignore long_method,high_complexity
function migrateLegacyShape(input: unknown) {
  // …
}
```

### Raise a single threshold

```rust
// cha:set long_method=120
fn parse_protocol_frame(buf: &[u8]) -> Frame {
    // 95 lines — over the 50-line default but under our 120-line budget.
}
```

### Raise every numeric threshold

```python
# cha:set threshold=200
def state_machine_step(event):
    # Long, branchy, intentionally so. Don't warn on length or complexity.
    ...
```

The finding is dropped if `actual_value < threshold` after the override; otherwise it still reports.
