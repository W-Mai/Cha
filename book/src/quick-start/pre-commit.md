# Quick Start: Pre-commit

Run Cha on staged files before each commit, blocking the commit on warning-or-worse findings.

## Setup

Add to `.pre-commit-config.yaml`:

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/W-Mai/Cha
    rev: v1.19.0
    hooks:
      - id: cha-analyze
```

Then:

```bash
pre-commit install
```

The hook id `cha-analyze` is defined in [`.pre-commit-hooks.yaml`](https://github.com/W-Mai/Cha/blob/main/.pre-commit-hooks.yaml) and runs:

```
cha analyze --diff --fail-on warning
```

## What gets analyzed

- `--diff` — only files changed in the working tree are analyzed. Untouched files don't slow the hook down.
- `--fail-on warning` — the commit is blocked if any finding has severity `warning` or `error`. Hints don't block.
- `types_or` is set to `c, c++, rust, python, go, ts, javascript`, so the hook only fires when staged files include one of these languages.
- `pass_filenames` is `false`: Cha walks the diff itself rather than receiving a file list from pre-commit, so the gitignore-aware behaviour stays consistent with `cha analyze`.

## Troubleshooting

**Too noisy on day one.** A fresh install on a legacy codebase will fire on every commit. Two options:

```bash
# Option A: only block on errors, downgrade warnings to advisory
# Edit .pre-commit-config.yaml -> args: [--fail-on, error]
```

```bash
# Option B: snapshot existing findings as baseline; only new findings block
cha baseline
git add .cha/baseline.json
```

Then in `.pre-commit-config.yaml`:

```yaml
- id: cha-analyze
  args: [--diff, --fail-on, warning, --baseline, .cha/baseline.json]
```

See [Suppressing legacy issues](../recipes/suppress-legacy.md) for the full baseline workflow.

**Hook is slow.** First run on a fresh checkout is uncached; subsequent runs hit the warm cache and finish in well under a second on most projects. If a single file is repeatedly slow, profile with `cha analyze --no-cache <path>` to confirm it's parsing rather than the hook framework.

**Hook ignores my language.** Pre-commit's `types_or` filter is conservative. Add the file extension to your hook config or open an issue if a supported language isn't matching.
