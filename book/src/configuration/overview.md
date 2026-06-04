# Overview

Cha reads `.cha.toml` from your project root. Generate a starter file with:

```
cha init
```

## Where the config lives

`Config::load_for_file` walks from each analyzed file's directory up to the project root and merges every `.cha.toml` it finds along the way. Closer-to-the-file values win; root values are the base. This lets a sub-package override only the keys it cares about.

For most projects a single `.cha.toml` at the repo root is enough.

## Top-level keys

### `plugins`

Per-plugin configuration. Every plugin is enabled by default; set `enabled = false` to skip it. Other keys under `[plugins.<name>]` are forwarded to the plugin as options.

```toml
[plugins.length]
enabled = true
max_function_lines = 50
max_class_lines = 200

[plugins.coupling]
max_imports = 15
```

Numeric thresholds scale by `strictness` (see below); string and bool options pass through as-is. The full list of plugin keys is in [Configuration keys](../reference/config-keys.md).

### `exclude`

Glob patterns for paths to skip during analysis. Applied on top of `.gitignore`.

```toml
exclude = ["*/tests/fixtures/*", "vendor/*", "**/generated/**"]
```

### `debt_weights`

Minutes-per-finding used by the analyze summary's tech-debt total. Defaults: `hint = 5`, `warning = 15`, `error = 30`.

```toml
[debt_weights]
hint = 5
warning = 15
error = 30
```

### `strictness`

Multiplier applied to every numeric plugin threshold:

- `"relaxed"` — 2.0× (thresholds doubled, more lenient)
- `"default"` — 1.0×
- `"strict"` — 0.5× (thresholds halved)
- A custom float, e.g. `0.7`

```toml
strictness = "strict"
# or
strictness = 0.7
```

`get_usize` clamps the scaled result to a minimum of 1, so `strict` mode never produces zero thresholds.

### `languages`

Per-language overrides on top of the global plugin config and the builtin language profiles. The two sub-keys are `plugins` (same shape as the top-level `plugins` table) and `disabled_smells` (a list of smell names).

```toml
[languages.c.plugins.naming]
enabled = false

[languages.c.plugins.length]
max_function_lines = 80

[languages.python]
disabled_smells = ["naming_too_short"]
```

Builtin language profiles (currently for `c` and `cpp`) apply first; user overrides win. See [Strictness and presets](presets.md).

### `disabled_smells`

A flat list of smell names to suppress globally. Use this when a plugin emits multiple smells but you only want to silence some of them.

```toml
disabled_smells = ["naming_too_short", "todo_comment"]
```

For more surgical, per-item suppression, use [inline directives](inline-directives.md) instead.

### `layers`

Manual module and tier definitions for `cha layers`. Skip this section to let cha auto-infer layers from import dependencies.

```toml
[layers.modules]
domain = ["src/domain/**"]
service = ["src/service/**"]
controller = ["src/controller/**"]

[[layers.tiers]]
name = "core"
modules = ["domain"]

[[layers.tiers]]
name = "app"
modules = ["service", "controller"]
```

## Related pages

- [Inline directives](inline-directives.md) — `// cha:ignore` and `// cha:set` in source files.
- [Strictness and presets](presets.md) — strictness levels and builtin language profiles.
- [Configuration keys](../reference/config-keys.md) — full reference of every key and its default.
