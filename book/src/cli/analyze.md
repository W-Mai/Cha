# analyze

Run code-smell detectors over source files and print findings.

`cha analyze` is the default workflow command. It walks the given paths (or the current directory), parses every supported file once, runs the configured plugins in parallel, applies any baseline / diff filtering, and prints findings in the requested format.

## Usage

`cha analyze [PATHS]... [FLAGS]`

Paths default to `.` (recursive, `.gitignore`-aware). Multiple paths are accepted.

## Examples

```bash
# Analyze the whole project, terminal output
cha analyze

# Analyze a path with JSON output, fail CI on any error-severity finding
cha analyze src/ --format json --fail-on error

# Only analyze files changed in the working tree
cha analyze --diff

# Analyze a piped diff (PR review)
gh pr diff | cha analyze --stdin-diff --fail-on warning

# Run a single detector
cha analyze --plugin complexity

# Run several detectors
cha analyze --plugin complexity,naming,coupling

# Only show couplers and security findings
cha analyze --focus couplers,security

# Generate an HTML report
cha analyze --format html --output report.html

# Skip cached results, re-parse every file
cha analyze --no-cache

# Only report findings missing from the baseline
cha analyze --baseline .cha/baseline.json

# Crank thresholds tighter (0.5× of every default)
cha analyze --strictness strict

# Show only the 20 worst findings
cha analyze --top 20

# Show every finding (no aggregation, terminal only)
cha analyze --all
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--format <FMT>` | `terminal` | Output format: `terminal`, `json`, `llm`, `sarif`, `html`. |
| `--fail-on <LEVEL>` | _(none)_ | Exit with code 1 if any finding has severity ≥ `LEVEL`: `hint`, `warning`, `error`. |
| `--diff` | `false` | Only analyze files changed in `git diff` (unstaged). |
| `--stdin-diff` | `false` | Read a unified diff from stdin and only analyze the changed files / lines. |
| `--plugin <NAMES>` | _(all)_ | Comma-separated list of plugin names to run. Restricts analysis to those plugins only. |
| `--no-cache` | `false` | Disable the on-disk cache and force a full re-analysis. |
| `--baseline <PATH>` | _(none)_ | Suppress findings already recorded in the baseline file. |
| `--output, -o <PATH>` | _(stdout)_ | Write the report to a file. Most useful with `--format html`. |
| `--strictness <S>` | _(config)_ | Override `.cha.toml` strictness. Accepts `relaxed` (2×), `default` (1×), `strict` (0.5×), or any custom float (e.g. `0.7`). |
| `--all` | `false` | Show every finding, no aggregation. Terminal format only. |
| `--top <N>` | _(none)_ | Show only the top N most severe findings. Terminal format only. |
| `--focus <CATS>` | _(all)_ | Comma-separated category filter: `bloaters`, `oo_abusers`, `change_preventers`, `dispensables`, `couplers`, `security`. |

The `--config <PATH>` global flag also applies — pass it before `analyze` to load a non-default `.cha.toml`.

## Output

- `terminal` — grouped, colourised, tech-debt summary at the bottom.
- `json` — structured findings + per-file health scores. The schema is published via `cha schema`.
- `sarif` — SARIF 2.1.0, suitable for GitHub code-scanning.
- `llm` — compact, model-friendly text context.
- `html` — single-file report with source previews; pair with `--output`.

## Exit code

`0` unless `--fail-on` is set and at least one finding meets or exceeds the requested severity, in which case `1`.

## See also

- [baseline](./baseline.md) — generate the file consumed by `--baseline`.
- [calibrate](./calibrate.md) — derive per-project thresholds.
- [Configuration overview](../configuration/overview.md)
- [Smells reference](../plugins/reference.md)
- [Output formats](../output/index.md)
