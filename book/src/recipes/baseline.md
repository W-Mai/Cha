# Baseline workflow

A baseline is a snapshot of every finding present at a chosen moment. `cha analyze --baseline <path>` filters out anything in that snapshot, so CI only fails on findings introduced after the snapshot was taken.

This recipe is the day-to-day rhythm: generate, compare, refresh.

## Generate

Once, on the main branch, after deciding "everything currently here is grandfathered":

```bash
cha baseline
```

Default location: `.cha/baseline.json`. Override with `-o`:

```bash
cha baseline -o .cha/legacy-2026-Q1.json
```

The file is a list of fingerprints — `(path, smell name, normalised location)`. Small, diff-friendly, commit it.

```bash
git add .cha/baseline.json
git commit -m "Cha baseline at adoption"
```

## Use in CI

```bash
cha analyze --baseline .cha/baseline.json --fail-on warning
```

Fingerprints in the baseline are silent. Everything else surfaces. New issue on a touched line → CI fails. Old issue carried over → CI passes (silently).

A GitHub Actions step:

```yaml
- name: cha
  run: cha analyze --baseline .cha/baseline.json --fail-on warning
```

## Compare

After a few weeks, see what changed:

```bash
cha baseline -o /tmp/now.json
diff -u .cha/baseline.json /tmp/now.json | less
```

Lines marked `-` (in baseline but not now) are findings that disappeared — debt paid. Lines marked `+` (in now but not baseline) shouldn't exist if `--baseline` was respected — investigate the CI config.

## Refresh

When the team has paid down enough debt that the baseline is mostly stale, regenerate:

```bash
cha baseline                 # overwrites .cha/baseline.json
git diff .cha/baseline.json  # see what shrank
git commit -am "Refresh Cha baseline (-32 entries)"
```

Refresh on a regular cadence (quarterly is common) or whenever a major refactor lands. The commit message should record how many entries dropped — that's a real number for retros.

## Multi-package

In a monorepo, give each package its own baseline:

```bash
for pkg in packages/*/; do
  ( cd "$pkg" && cha baseline )
done
```

Each `.cha/baseline.json` lives next to its package. CI runs Cha per-package with its own baseline. See [CI on a monorepo](./monorepo-ci.md).

## What baseline does *not* fix

- **Wrong rules** — if a smell is firing on code you genuinely don't want flagged, don't bury it in baseline. Tune the threshold in `.cha.toml`, or use [inline directives](../configuration/inline-directives.md) for the specific item, or disable the plugin if it's noise everywhere.
- **Drift** — baseline silences existing findings. If new code is producing the same kind of finding, baseline lets it through *only* if its fingerprint matches an existing entry exactly. Identical-looking findings on new code surface normally.

## See also

- [`cha baseline`](../cli/baseline.md)
- [Suppress in legacy code](./suppress-legacy.md)
- [CI on a monorepo](./monorepo-ci.md)
