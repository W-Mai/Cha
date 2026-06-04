# Calibrate to your codebase

Cha's defaults (`max_function_lines=50`, `complexity warn=10`) are guesses informed by the Fowler / SonarSource literature. They're approximately right for greenfield code and approximately wrong for almost everything else.

`cha calibrate` samples your project's actual distribution and proposes thresholds at the 90th and 95th percentiles. The 90th becomes the warning threshold, the 95th becomes the error threshold. Code that's more complex than 95% of the codebase fails CI; code more complex than 90% gets a warning; everything else is silent.

## When to run it

- Adopting Cha on an existing codebase.
- After a quarter or two of growth (the distribution shifts).
- When the team disagrees about whether a given finding is "really a problem" — let the data answer.

## The workflow

```bash
cha calibrate
```

Sample output:

```
Analyzed 1284 functions across 73 files.

Metric                    Warning(P90) Error(P95)
────────────────────────────────────────────────
long_method                       42         71
high_complexity                    8         13
cognitive_complexity              11         19
```

Read it as: "90% of your functions are ≤ 42 lines; 95% are ≤ 71 lines". The suggestion: warn at 42, error at 71.

If those numbers feel right, save them:

```bash
cha calibrate --apply
```

This writes `.cha/calibration.toml` with the chosen thresholds **and** the underlying P50 / P75 / P90 / P95 distribution for every metric. `cha analyze` picks it up automatically on the next run.

## Precedence

The thresholds Cha uses, in order of strength:

1. Per-item inline directive (`// cha:set max_function_lines=200`).
2. `.cha.toml` `[plugins.<name>]` settings.
3. `.cha/calibration.toml` (written by `cha calibrate --apply`).
4. Built-in defaults.

If a value sits in `.cha.toml`, calibrate's number is ignored. This is deliberate — explicit team agreement beats automated sampling.

## Reading the distribution file

`.cha/calibration.toml` looks like:

```toml
[long_method]
warning = 42
error = 71
p50 = 18
p75 = 31
p90 = 42
p95 = 71

[high_complexity]
warning = 8
error = 13
p50 = 3
p75 = 5
p90 = 8
p95 = 13

[cognitive_complexity]
warning = 11
error = 19
p50 = 4
p75 = 7
p90 = 11
p95 = 19
```

The percentiles let you tune by hand. If P90 = 42 lines and P95 = 71 lines, the gap means a long tail of unusually long functions. Lowering `error` to 60 catches that tail; raising `warning` to 50 lets normal functions breathe.

## Strictness multiplier

`strictness` in `.cha.toml` scales every threshold (calibrated or not) by a factor:

```toml
strictness = "strict"   # 0.5×
strictness = "default"  # 1.0×
strictness = "relaxed"  # 2.0×
strictness = 0.7        # custom
```

Calibrate, then dial via `strictness` if the team wants the same shape but tighter overall.

## Limitations

`calibrate` only samples function-level metrics. Class-level (`max_class_lines`, `max_class_methods`) and file-level (`max_file_lines`) thresholds aren't sampled — set them by hand or stay on defaults.

## See also

- [`cha calibrate`](../cli/calibrate.md)
- [Strictness and presets](../configuration/presets.md)
- [Configuration overview](../configuration/overview.md)
