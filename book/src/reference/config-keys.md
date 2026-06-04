# Configuration keys

Every key that `.cha.toml` understands. The structure is fixed by [`cha_core::Config`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/config.rs); unknown keys are ignored silently.

## Top-level

| Key | Type | Default | Effect |
|---|---|---|---|
| `exclude` | `Vec<String>` | `[]` | Glob patterns for paths to skip. Patterns match against the relative path from project root. `**` matches any depth. The walker also honours `.gitignore`, so excluding `node_modules/` is usually unnecessary. |
| `strictness` | `"relaxed"` / `"default"` / `"strict"` / `f64` | `"default"` | Multiplier applied to every threshold-based check. `relaxed = 2.0×`, `default = 1.0×`, `strict = 0.5×`, or any custom float (`0.7`). Affects integer thresholds; ratios pass through unchanged. |
| `disabled_smells` | `Vec<String>` | `[]` | Smell names to suppress globally. Use when one plugin emits multiple smells (`length` → `long_method` / `large_class` / `large_file`) and you only want some silenced. |
| `debt_weights` | table | see below | Tech-debt minutes per severity level. Used by `cha analyze` summary line. |
| `plugins` | table of tables | (defaults) | Per-plugin overrides. See [Per-plugin section](#per-plugin-section). |
| `languages` | table of tables | `{}` | Per-language overrides. See [Per-language section](#per-language-section). |
| `layers` | table | (empty) | Module / tier definitions for `cha layers`. See [Layers section](#layers-section). |

### `debt_weights`

```toml
[debt_weights]
hint = 5        # default 5
warning = 15    # default 15
error = 30      # default 30
```

Each value is minutes. The summary line shows total debt as `<n>h <n>m`.

## Per-plugin section

Every plugin reads its config from `[plugins.<name>]`. The structure is:

```toml
[plugins.<name>]
enabled = true             # default true; set false to disable
# ... plugin-specific keys
```

Plugin-specific keys vary; defaults match the values in each plugin's `Default` impl. The full table:

| `[plugins.<name>]` | Keys | Notes |
|---|---|---|
| `length` | `max_function_lines` (50), `max_class_methods` (10), `max_class_lines` (200), `max_file_lines` (500), `complexity_factor_threshold` (10.0) | Severity scales with how far the value overshoots the threshold. |
| `complexity` | `warn_threshold` (10), `error_threshold` (20) | Cyclomatic complexity. |
| `cognitive_complexity` | `threshold` (15) | Promotes to `Error` above `2 × threshold`. |
| `long_parameter_list` | `max_params` (5) | |
| `primitive_obsession` | `min_params` (3), `primitive_ratio` (0.8) | |
| `data_clumps` | `min_clump_size` (3), `min_occurrences` (3) | |
| `naming` | `min_name_length` (2), `max_name_length` (50) | |
| `api_surface` | `max_exported_ratio` (0.8), `max_exported_count` (20), `c_max_exported_ratio` (1.01), `c_max_exported_count` (30), `skip_c_headers` (true) | C-language overrides because headers expose by design. |
| `god_class` | `max_external_refs` (5), `min_wmc` (47), `min_tcc` (0.33) | ATFD / WMC / TCC metrics from Lanza & Marinescu. |
| `brain_method` | `min_lines` (65), `min_complexity` (4), `min_external_refs` (7) | |
| `coupling` | `max_imports` (15) | Promotes to `Error` above `2 × max_imports`. |
| `hub_like_dependency` | `max_imports` (20) | |
| `feature_envy` | `min_refs` (3), `external_ratio` (0.7) | |
| `middle_man` | `min_methods` (3), `delegation_ratio` (0.5) | |
| `message_chain` | `max_depth` (3) | |
| `inappropriate_intimacy` | (no thresholds) | Detects bidirectional imports. |
| `layer_violation` | `layers = "domain:0,service:1,..."` | String form: `name:rank,name:rank,...`. Lower rank may not import higher rank. |
| `async_callback_leak` | (no thresholds) | Detects raw `JoinHandle` / `Future` / `Channel` in public signatures. |
| `switch_statement` | `max_arms` (8) | |
| `temporary_field` | `min_methods` (3), `max_usage_ratio` (0.3) | |
| `refused_bequest` | `min_override_ratio` (0.5), `min_methods` (3) | |
| `design_pattern` | `strategy_min_arms` (4), `state_min_arms` (3), `builder_min_params` (7), `builder_alt_min_params` (5), `builder_alt_min_optional` (3), `null_object_min_count` (3), `template_min_self_calls` (3), `template_min_methods` (4), and several keyword lists | Six patterns; thresholds independent. |
| `shotgun_surgery` | `min_co_changes` (5), `max_commits` (100) | Reads `git log`. |
| `divergent_change` | `min_distinct_reasons` (4), `max_commits` (50) | Reads `git log`. |
| `dead_code` | `entry_points` (per-language defaults) | Functions in this list are never flagged as dead. |
| `duplicate_code` | `min_lines` (10) | AST-hash comparison. |
| `comments` | `max_comment_ratio` (0.3), `min_lines` (10) | |
| `lazy_class` | `max_methods` (1), `max_lines` (10) | |
| `data_class` | `min_fields` (2) | |
| `speculative_generality` | (no thresholds) | Interface / trait with ≤ 1 implementation. |
| `todo_tracker` | (no thresholds) | TODO / FIXME / HACK / XXX. HACK and XXX promote to Warning. |
| `hardcoded_secret` | (regex set) | API keys, tokens, passwords, JWTs. |
| `unsafe_api` | (call set) | `eval`, `exec`, `system`, `strcpy`, `gets`, `unsafe`, `innerHTML`, `dangerouslySetInnerHTML`, etc. |
| `error_handling` | `max_unwraps_per_function` (3) | Empty `catch` always flagged. |

Authoritative defaults live in each plugin's `Default for <Analyzer>` impl in [`cha-core/src/plugins/`](https://github.com/W-Mai/Cha/tree/main/cha-core/src/plugins).

## Per-language section

```toml
[languages.<lang>]
disabled_smells = []
[languages.<lang>.plugins.<name>]
# ... same keys as [plugins.<name>]
```

`<lang>` is the language ID Cha attaches to the file: `python`, `typescript`, `tsx`, `rust`, `go`, `c`, `cpp`. Per-language values **override** global values; they don't merge. The C preset turns off `naming`, `lazy_class`, `data_class`, `design_pattern` and bumps `length.max_function_lines` to 80.

Example:

```toml
[languages.c.plugins.naming]
enabled = false

[languages.c.plugins.length]
max_function_lines = 80

[languages.python.plugins.long_parameter_list]
max_params = 8                          # Python tolerates more **kwargs
```

## Layers section

For `cha layers` and the `layer_violation` smell:

```toml
[layers]
modules = { domain = ["src/domain/**"], service = ["src/service/**"], controller = ["src/handlers/**"] }

[[layers.tiers]]
name = "data"
modules = ["domain"]

[[layers.tiers]]
name = "logic"
modules = ["service"]

[[layers.tiers]]
name = "api"
modules = ["controller"]
```

Tier order in the file = bottom to top. Lower tiers may not import higher tiers. The simpler `layer_violation` plugin uses the inline `layers = "..."` form on `[plugins.layer_violation]` instead.

## Inline directives

Override config per-item directly in source. See [Inline directives](../configuration/inline-directives.md):

```rust
// cha:ignore                        — suppress all rules for the next item
// cha:ignore long_method            — suppress one rule
// cha:ignore long_method,complexity — suppress multiple
// cha:set long_method=100           — raise the long_method threshold for the next item
// cha:set threshold=200             — raise the threshold for every threshold-based rule on the next item
```

Works with `//`, `#`, and `/* */` comment styles.

## See also

- [Configuration overview](../configuration/overview.md)
- [Strictness and presets](../configuration/presets.md)
- [Inline directives](../configuration/inline-directives.md)
- [JSON Schema](./json-schema.md) — for the *output* schema, not the config schema.
- [`cha-core/src/config.rs`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/config.rs) — authoritative source.
