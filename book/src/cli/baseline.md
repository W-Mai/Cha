# baseline

Snapshot the current set of findings so future runs only report new issues.

`cha baseline` runs the same analysis as `cha analyze` and writes a fingerprint of every finding to a JSON file. `cha analyze --baseline <path>` then suppresses any finding whose fingerprint already exists in that file. New findings (added since the snapshot was taken) still surface.

This is the standard way to introduce Cha to a legacy codebase without drowning CI in pre-existing issues.

## Usage

`cha baseline [PATHS]... [--output <PATH>]`

Paths default to `.`.

## Examples

```bash
# Snapshot today's findings into the default location
cha baseline

# Choose where the file goes
cha baseline --output .cha/legacy.json

# CI: fail only on findings introduced after the baseline
cha analyze --baseline .cha/baseline.json --fail-on warning
```

A typical workflow:

1. `cha baseline` once, on the main branch, after deciding "everything currently here is grandfathered".
2. Commit `.cha/baseline.json`.
3. CI runs `cha analyze --baseline .cha/baseline.json --fail-on warning` on every PR.
4. Periodically delete entries from the baseline file (or regenerate it) as the team pays down debt.

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--output, -o <PATH>` | `.cha/baseline.json` | Where to write the baseline JSON. Parent directories are created if missing. |

The global `--config <PATH>` flag also applies; the same `exclude` patterns as `analyze` are respected.

## Output

```
Baseline saved to .cha/baseline.json (147 findings)
```

The file itself is a list of fingerprints (file path, smell name, normalised location). It is small enough to commit and diff-friendly.

## See also

- [analyze](./analyze.md) — how `--baseline` is consumed.
- [Baseline workflow recipe](../recipes/baseline.md)
- [Suppress in legacy code](../recipes/suppress-legacy.md)
