# Strictness and presets

Two mechanisms tune Cha's defaults: the global **strictness factor** (a numeric multiplier applied to every threshold) and **per-language presets** (builtin profiles plus user overrides).

## Strictness

`strictness` in `.cha.toml` (or the `--strictness` CLI flag) multiplies every numeric plugin threshold:

| Value | Factor | Effect |
|-------|--------|--------|
| `"relaxed"` | 2.0× | Thresholds doubled — more lenient. |
| `"default"` | 1.0× | Plugin defaults as-shipped. |
| `"strict"` | 0.5× | Thresholds halved — fewer findings escape. |
| Any float, e.g. `0.7` | as written | Custom multiplier. |

```toml
strictness = "strict"
# or
strictness = 0.7
```

`relaxed`, `default`, and `strict` are the named levels; anything else parses as a float. The scaled result is clamped to a minimum of 1, so even `strict` never collapses a threshold to zero.

Strictness applies to integer plugin options (function-line caps, complexity thresholds, parameter counts, …). Ratios such as `external_ratio` or `primitive_ratio` are read as-written — they aren't scaled.

## Per-language presets

Cha ships builtin profiles for these languages: `c`, `cpp`, `python`, `typescript`, `rust`, `go`. Inspect them with:

```bash
cha preset list           # which languages have profiles, and how many smells each disables
cha preset show c         # full resolved config for C: plugins, smells, strictness factor
cha preset show rust
```

Currently only the `c` / `cpp` profile changes defaults from the global plugin set. Profiles for the other languages exist in the listing but apply no overrides today.

### The C / C++ profile

C is procedural, so the OO-leaning detectors are turned off by default:

- **Plugins disabled** (no smells emitted at all): `naming`, `lazy_class`, `data_class`.
- **Individual smells disabled** (the plugin still runs, but these specific smells are filtered out): `builder_pattern`, `null_object_pattern`, `strategy_pattern`, `data_clumps`.

The profile also tunes the size and coupling thresholds upward, since C codebases tend to have longer functions and more includes than typical Rust/TypeScript projects:

| Plugin | Option | Value |
|--------|--------|-------|
| `length` | `max_function_lines` | 100 |
| `length` | `max_file_lines` | 2000 |
| `length` | `max_class_lines` | 400 |
| `complexity` | `warn_threshold` | 15 |
| `complexity` | `error_threshold` | 30 |
| `cognitive_complexity` | `threshold` | 25 |
| `coupling` | `max_imports` | 25 |
| `long_parameter_list` | `max_params` | 7 |

## User overrides

Anything in `[languages.<lang>]` overrides the builtin profile for that language. The shape is identical to the global config:

```toml
# Re-enable naming on C with a relaxed minimum length.
[languages.c.plugins.naming]
enabled = true
min_name_length = 3

# Drop a specific smell for Python without disabling any plugin entirely.
[languages.python]
disabled_smells = ["naming_too_short"]

# Tighten one threshold on Rust without changing the global strictness.
[languages.rust.plugins.length]
max_function_lines = 40
```

User keys win over the builtin profile, so re-enabling a builtin-disabled plugin works without further ceremony.

To see the resolved config for a language after your overrides:

```bash
cha preset show <language>
```

The output reports the effective strictness factor, every plugin that would run, plugins disabled by the builtin profile, and any extra `disabled_smells` you added.

## Related

- [Configuration overview](overview.md) — top-level keys.
- [Inline directives](inline-directives.md) — per-item overrides without editing config.
- [`cha preset`](../cli/preset.md) — command reference.
