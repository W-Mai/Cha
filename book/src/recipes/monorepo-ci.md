# CI on a monorepo

One repo, many packages. A PR usually touches one or two of them, but a naive `cha analyze` walks the whole tree and bills you for the rest.

Two strategies, often combined.

## Strategy 1: analyze only changed files

`cha analyze --diff` runs against files modified in the working tree. In a PR pipeline, point it at the PR diff:

```bash
# Local: files changed against the working tree
cha analyze --diff

# CI: a PR diff piped in
gh pr diff "$PR_NUMBER" | cha analyze --stdin-diff --fail-on warning
```

`--stdin-diff` accepts unified-diff format on stdin. The same `.cha.toml` applies; only the file list narrows.

A GitHub Actions step:

```yaml
- name: cha (PR diff)
  run: |
    gh pr diff ${{ github.event.pull_request.number }} \
      | cha analyze --stdin-diff --fail-on warning
  env:
    GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

Push events get the full analyze on the affected paths:

```yaml
- name: cha (push)
  run: cha analyze --fail-on warning
```

## Strategy 2: per-package config

If the monorepo has `packages/api`, `packages/web`, `packages/shared`, give each package its own `.cha.toml` and run Cha per-package:

```bash
for pkg in packages/*/; do
  ( cd "$pkg" && cha analyze --fail-on warning ) || exit 1
done
```

Each `.cha.toml` is independent. The shared library can keep stricter `max_function_lines`; the experimental package can relax `complexity`. There's no inheritance, deliberately — config drift between packages is a feature when packages have different shapes.

## Combine: baseline per package + diff in PR

The pattern that scales:

1. Each package has its own `.cha/baseline.json`, generated once when Cha was adopted.
2. PR runs use both `--baseline` and `--stdin-diff`:

   ```bash
   cha analyze --stdin-diff \
     --baseline .cha/baseline.json \
     --fail-on warning < diff.patch
   ```
3. Push to main runs the full analyze with `--baseline` only — catches drift even when no individual file changed.

Old findings are silenced by the baseline. New findings on changed lines fail CI. Untouched code outside the diff is skipped.

## Caching

Cha caches parsed AST + finding results to `.cha/cache/`. In CI, cache that directory between runs:

```yaml
- uses: actions/cache@v4
  with:
    path: .cha/cache
    key: cha-${{ hashFiles('**/Cargo.lock', '**/package-lock.json', '**/go.sum') }}
```

The cache key picks up dependency changes; finer-grained keys aren't worth the complexity.

## See also

- [Baseline workflow](./baseline.md)
- [Suppress in legacy code](./suppress-legacy.md)
- [`cha analyze`](../cli/analyze.md)
