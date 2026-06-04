# Quick Start: CLI

Five minutes from `cha` not installed to a tightened threshold.

## 1. Install

macOS / Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.sh | sh
```

Windows (PowerShell):

```powershell
powershell -c "irm https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.ps1 | iex"
```

Homebrew:

```bash
brew install W-Mai/cellar/cha-cli
```

See [Installation](../install.md) for all options.

## 2. First analyze

From the project root:

```bash
cha analyze
```

This walks the working directory recursively, honours `.gitignore`, runs all 34 built-in detectors, and prints a terminal report. On a fresh project you'll see something like:

```
src/handlers.rs:142:8  warning  long_method
  Function `process_request` is 87 lines (threshold: 50)

src/state.rs:23:1      warning  god_class
  Class `AppState` has WMC 53, ATFD 8, TCC 0.21

— 2 warnings, 0 errors, debt: 30m
```

The trailing line is the summary: how many findings of each severity, plus an estimated tech-debt time computed from `[debt_weights]` in your config.

## 3. Read findings

Three things matter per finding:

- **Severity** — `hint` / `warning` / `error`. Severity drives `--fail-on` in CI.
- **Smell name** — the detector that fired. Look it up in the [plugin reference](../plugins/reference.md) for what it means and how to fix it.
- **Threshold vs. actual** — the message tells you the configured limit and the value that breached it. That's your lever.

For deeper detail on a single file:

```bash
cha parse src/handlers.rs
```

prints the parsed structure (functions, classes, imports) Cha sees, which is helpful when a finding doesn't make sense.

## 4. Tighten or relax thresholds

If a smell is firing too often, two paths:

**Tighten in `.cha.toml`** — keep the rule, change the number:

```toml
[plugins.length]
max_function_lines = 80   # was 50
```

**Suppress per-item** — keep the rule, exclude one site:

```rust
// cha:ignore long_method
fn process_request(...) { ... }
```

If you have no idea what numbers to pick, let Cha pick them from your project's distribution:

```bash
cha calibrate            # print suggested thresholds
cha calibrate --apply    # write them to .cha/calibration.toml
```

P90 of each metric becomes the warning threshold, P95 becomes the error threshold. See [calibrate](../cli/calibrate.md).

## 5. Wire into CI

Most projects pick one of:

- [Pre-commit hook](./pre-commit.md) — block locally, before push.
- [GitHub Action](./github-actions.md) — block in PR, upload SARIF to Code Scanning.
- [Editor / LSP](./editor.md) — see findings while you type.
