# trend

Replay the last N commits and report how the total finding count moves over time.

`cha trend` checks out each of the last N commits in turn, runs the analyser, and records the total number of findings. The result is a time series showing whether code health is improving, regressing, or steady.

This is the historical companion to `cha analyze`: where `analyze` answers "what's wrong now?", `trend` answers "are we paying down or accumulating debt?"

## Usage

`cha trend [FLAGS]`

`trend` analyses the current git repository — it does not take path arguments.

## Examples

```bash
# Default: last 10 commits, terminal output
cha trend

# Look further back
cha trend -c 50

# Machine-readable
cha trend -c 30 --format json
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `-c, --count <N>` | `10` | Number of recent commits to replay. |
| `--format <FMT>` | `terminal` | Output format: `terminal` or `json`. |

The global `--config <PATH>` flag also applies; the same plugin set and config as `analyze` is used at each commit.

## Output

Terminal output is one row per commit, oldest at the top:

```
abc1234  2024-09-12  +0   -3   147 findings
def5678  2024-09-13  +5   +0   152 findings
...
```

`+`/`-` columns show how the count changed from the previous commit. JSON returns the same data as a list.

## Caveats

- Each commit is checked out and analysed in turn — `trend -c 50` on a large project takes time. Cache hits across commits help, but the first commit always cold-starts.
- The current working tree must be clean. Uncommitted changes block the checkout loop.
- Configuration is read at HEAD: if `.cha.toml` changed during the window, older commits are still analysed under today's rules. That is usually what you want, but be aware.

## See also

- [hotspot](./hotspot.md) — same git source, file-axis view.
- [analyze](./analyze.md) — the per-commit analysis being replayed.
