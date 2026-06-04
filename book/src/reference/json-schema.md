# JSON Schema

`cha schema` prints a [JSON Schema 2020-12](https://json-schema.org/draft/2020-12/release-notes) document describing the structure of `cha analyze --format json` output. Use it to validate Cha output, generate types in another language, or wire IDE auto-completion when a tool consumes Cha findings.

This is **not** a schema for `.cha.toml` — Cha's config has no published schema. For configuration keys, see [Configuration keys](./config-keys.md).

## Generate

```bash
cha schema > cha-findings.schema.json
```

The output is a `Vec<Finding>` schema, derived from the `Finding` struct in [`cha-core/src/model.rs`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/model.rs) via the [`schemars`](https://crates.io/crates/schemars) crate. Every release re-runs the derive, so the schema always matches the current `Finding` shape.

## Use it

### Validate JSON output

```bash
cha analyze --format json > findings.json
cha schema > cha-findings.schema.json

# Pick a JSON Schema validator; check-jsonschema is one option:
check-jsonschema --schemafile cha-findings.schema.json findings.json
```

Exit code 0 means the output conforms to the schema. Useful in CI when a downstream tool is parsing Cha output.

### Wire IDE auto-completion

For tools or scripts that read `findings.json` directly, point your editor's JSON support at the schema. In VS Code:

```jsonc
// .vscode/settings.json
{
  "json.schemas": [
    {
      "fileMatch": ["**/findings.json"],
      "url": "./cha-findings.schema.json"
    }
  ]
}
```

In editors using [`schemastore.org`](https://schemastore.org) — Helix, Neovim with `efm-langserver`, etc. — add a custom mapping. We don't publish to schemastore yet, so the file path is local.

### Generate types in another language

[`quicktype`](https://quicktype.io) consumes JSON Schema and emits TypeScript, Python, Java, C#, Go, Rust, etc.:

```bash
quicktype --src-lang schema cha-findings.schema.json -o ChaFindings.ts
```

The output is a typed dataclass / interface that mirrors `Finding`. Useful when you're writing a dashboard, exporter, or LSP-adjacent tool that consumes Cha output.

## What's in a Finding

The schema describes one of these per analysis result:

```json
{
  "smell_name": "long_method",
  "category": "Bloaters",
  "severity": "Warning",
  "location": {
    "path": "src/handlers.rs",
    "start_line": 142,
    "start_col": 8,
    "end_line": 198,
    "end_col": 1,
    "name": "process_request"
  },
  "message": "Function `process_request` is 87 lines (threshold: 50)",
  "suggested_refactorings": ["Extract Method"],
  "actual_value": 87.0,
  "threshold": 50.0,
  "risk_score": 1.74
}
```

| Field | Description |
|---|---|
| `smell_name` | Smell ID, e.g. `long_method`. Multiple plugins can emit the same smell name only if they coordinate (none currently do). |
| `category` | One of `Bloaters` / `Couplers` / `OOAbusers` / `ChangePreventers` / `Dispensables` / `Security`. Drives `--focus` and grouping in output. |
| `severity` | `Hint` / `Warning` / `Error`. Drives `--fail-on`. |
| `location` | File path + 1-based line range + 0-based column range. `name` is the offending symbol when a single one applies (function name, class name). |
| `message` | Human-readable, includes the threshold and actual value. |
| `suggested_refactorings` | Free-form labels referencing Fowler's catalog (`"Extract Method"`, `"Replace Conditional with Polymorphism"`, etc.). |
| `actual_value` / `threshold` | Numeric metric and the limit it crossed. Both nullable for non-threshold smells (e.g. `unsafe_api`). |
| `risk_score` | Severity × overshoot × structural compounding factor. Used by `cha trend` to rank issues. Nullable when not applicable. |

The schema captures these as required vs optional, with the right value enums on `category` and `severity`.

## Output formats that don't follow this schema

`cha analyze --format json` is the only output that conforms. The others have their own shapes:

- **`--format sarif`** follows [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html). Use a SARIF tool, not `cha schema`.
- **`--format html`** is rendered HTML — no schema applies.
- **`--format llm`** is markdown intended for LLM context — no schema applies.
- **`--format terminal`** is for humans.

## See also

- [`cha init` / `cha schema`](../cli/init.md) — the `cha schema` subcommand lives on the same page as `cha init`.
- [JSON output](../output/json.md) — what the JSON output looks like in practice.
- [Configuration keys](./config-keys.md) — for `.cha.toml`, not finding output.
- [`Finding` struct](https://github.com/W-Mai/Cha/blob/main/cha-core/src/model.rs) — authoritative source.
