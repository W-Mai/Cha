# calibrate

Suggest thresholds for `long_method`, `high_complexity`, and `cognitive_complexity`
from project statistics. The 90th percentile is used as the warning threshold and the
95th as the error threshold.

## Usage

```
cha calibrate [paths]... [--apply]
```

`paths` defaults to the current directory. Only the function-level metrics are
sampled — files and classes are not currently calibrated.

## Examples

```bash
# Print suggested thresholds and the underlying P50 / P75 / P90 / P95 distribution.
cha calibrate

# Calibrate over a subdirectory only.
cha calibrate src/

# Save thresholds to .cha/calibration.toml; future `cha analyze` runs pick them up
# automatically (overrides defaults, but a value in .cha.toml still wins).
cha calibrate --apply
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--apply` | off | Write the suggested thresholds to `.cha/calibration.toml`. |

## Output

```
Analyzed N functions across M files.

Metric                    Warning(P90) Error(P95)
────────────────────────────────────────────────
long_method                       42         71
high_complexity                    8         13
cognitive_complexity              11         19
```

`--apply` additionally writes a TOML file with the full P50 / P75 / P90 / P95
distribution alongside the chosen thresholds. The file is human-editable; delete it
to revert to the built-in defaults.

## See also

- [`analyze`](./analyze.md) — picks up calibrated thresholds when present.
- [Strictness and presets](../configuration/presets.md) — the multiplier that scales
  every threshold.
- [Calibrate to your codebase](../recipes/calibrate.md) — full workflow recipe.
