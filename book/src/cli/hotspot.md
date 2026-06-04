# hotspot

Rank files by `change_frequency × complexity` to surface refactoring targets.

A file that's complicated *and* changes often is where bugs live and where engineering time is spent. `cha hotspot` reads the last N commits from `git log`, multiplies each file's change count by its current cyclomatic complexity, and prints the top N.

The git history is read once per invocation and cached for the run. Files outside the supported language set are excluded; `.gitignore` is respected.

## Usage

`cha hotspot [FLAGS]`

`hotspot` analyses the current git repository — it does not take path arguments.

## Examples

```bash
# Default: last 100 commits, top 20 files
cha hotspot

# Look further back, show fewer files
cha hotspot -c 500 -t 10

# JSON for further processing
cha hotspot -c 200 -t 10 --format json

# Pipe into a viewer
cha hotspot --format json | jq '.[] | select(.score > 100)'
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `-c, --count <N>` | `100` | Number of recent commits to read from `git log`. |
| `-t, --top <N>` | `20` | Show the top N files by score. |
| `--format <FMT>` | `terminal` | Output format: `terminal` or `json`. |

The global `--config <PATH>` flag also applies.

## Output

Terminal output sorts by score descending and shows commits, complexity, and the product:

```
Score   Commits  Complexity  File
  342       38           9   src/analyze.rs
  198       22           9   src/deps.rs
  ...
```

JSON returns the same data as a list of objects, suitable for piping into dashboards or other tools.

## Caveats

- Requires a git repository — files not under git are skipped.
- A renamed file resets its history unless git's rename detection picks it up.
- The `--count` window is an upper bound: a freshly-cloned shallow repo may have fewer commits available.

## See also

- [trend](./trend.md) — same data source, time-axis view of issue counts.
- [Calibrate to your codebase recipe](../recipes/calibrate.md)
