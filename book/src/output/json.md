# JSON

Machine-readable output for scripts, dashboards, and any tool that wants to consume Cha's findings without parsing terminal text.

## Sample output

```text
{
  "findings": [
    {
      "actual_value": 0.0,
      "category": "dispensables",
      "location": {
        "end_col": 16,
        "end_line": 8,
        "name": "FileEntry",
        "path": "cha-core/src/cache.rs",
        "start_col": 7,
        "start_line": 8
      },
      "message": "Class `FileEntry` has only 0 method(s) and 8 lines, consider Inline Class",
      "risk_score": 1.5,
      "severity": "hint",
      "smell_name": "lazy_class",
      "suggested_refactorings": ["Inline Class"],
      "threshold": 1.0
    }
    …
  ],
  "health_scores": [
    { "debt_minutes": 60, "grade": "C", "lines": 501, "path": "cha-core/src/cache.rs" }
  ]
}
```

(Captured with `cha analyze --format json cha-core/src/cache.rs`.)

The `analyze` command wraps findings in an envelope `{ "findings": [...], "health_scores": [...] }`. Other commands that emit JSON (e.g. `cha trend --format json`) emit a bare array.

## When to use it

- Custom dashboards or PR-comment bots that ingest findings programmatically.
- Diffing two runs to see what changed.
- Exporting to a database for trend analysis beyond what `cha trend` and `cha hotspot` cover.
- Piping into `jq` for ad-hoc queries (`cha analyze --format json | jq '.findings[] | select(.severity=="error")'`).

## Notes / Gotchas

- The full schema lives at [`reference/json-schema.md`](../reference/json-schema.md) and is also embedded in the binary — run `cha schema` to print it to stdout, or pin it in your toolchain.
- `risk_score` is filled in **after** analysis by the prioritisation pass; it may be absent on findings that haven't been ranked.
- `actual_value`, `threshold`, and `risk_score` are nullable doubles — handle missing values when consuming the data.
- Locations use **0-based** columns. SARIF translates these to 1-based; if you need 1-based columns from JSON, add 1 yourself.
- Severity values are lowercase strings: `hint`, `warning`, `error`.
