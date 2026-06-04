# Quick Start: GitHub Actions

Run Cha on every PR and push findings to the GitHub Code Scanning tab via SARIF.

## Minimal workflow

```yaml
# .github/workflows/cha.yml
name: cha

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read
  security-events: write   # required for SARIF upload

jobs:
  cha:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: W-Mai/Cha@v1.19.0
        with:
          fail-on: warning
          upload-sarif: true
```

That's the action block from the README, expanded into a full job. The action installs the matching `cha` release for the runner OS, runs analysis, and uploads SARIF to Code Scanning when `upload-sarif: true`.

## Inputs

| Input | Default | Notes |
|-------|---------|-------|
| `version` | `latest` | Pin a specific tag (e.g. `v1.19.0`) for reproducible runs. |
| `format` | `sarif` | One of `terminal`, `json`, `sarif`, `html`. SARIF is required for the Code Scanning tab. |
| `fail-on` | `error` | One of `hint`, `warning`, `error`. Sets the CI exit-code threshold. |
| `plugin` | (all) | Comma-separated list, e.g. `complexity,naming`. |
| `baseline` | (none) | Path to a `cha baseline` file. Only findings missing from the baseline are reported. |
| `path` | `.` | Directory to analyze. |
| `upload-sarif` | `true` | Upload via `github/codeql-action/upload-sarif@v3` when `format` is `sarif`. |

## Common combinations

**Block PRs on warnings, surface results in Code Scanning:**

```yaml
- uses: W-Mai/Cha@v1.19.0
  with:
    fail-on: warning
    upload-sarif: true
```

**Report only, don't block (advisory mode):**

```yaml
- uses: W-Mai/Cha@v1.19.0
  with:
    fail-on: error          # warnings won't fail the job
    upload-sarif: true      # but they still show up in Code Scanning
```

**Block only on new findings, ignore legacy:**

```yaml
- uses: W-Mai/Cha@v1.19.0
  with:
    fail-on: warning
    baseline: .cha/baseline.json
    upload-sarif: true
```

**Local terminal output for the Actions log only (no SARIF):**

```yaml
- uses: W-Mai/Cha@v1.19.0
  with:
    format: terminal
    fail-on: warning
    upload-sarif: false
```

## SARIF and Code Scanning

When `format: sarif` and `upload-sarif: true`, the action writes `cha-results.sarif` and forwards it to `github/codeql-action/upload-sarif@v3` with category `cha`. Findings then appear under **Security → Code scanning** with file/line annotations on the PR.

`security-events: write` permission is required on the job; without it the upload step silently skips. See [SARIF output](../output/sarif.md) for the schema Cha emits.
