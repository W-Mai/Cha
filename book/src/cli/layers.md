# layers

Infer architectural layers from the import graph and show how the project is stacked.

`cha layers` reads the same import edges as `cha deps --type imports`, runs a layering algorithm over them, and reports which directories sit at which level. It does *not* require any prior layer configuration — the output is a hypothesis derived from the code.

The optional `--save` flag writes the inferred layers back into `.cha.toml` so the `layer_violation` detector can enforce them on future analyses.

## Usage

`cha layers [PATHS]... [FLAGS]`

Paths default to `.`.

## Examples

```bash
# Print the inferred layering as a table
cha layers

# Render as a Mermaid diagram
cha layers --format mermaid

# Dependency Structure Matrix — a triangular view that highlights cycles
cha layers --format dsm

# Override the auto-detected directory depth (e.g. group by 2 path segments)
cha layers --depth 2

# Save the inferred layers into .cha.toml under [plugins.layer_violation]
cha layers --save
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--format <FMT>` | `dot` | Output format: `dot` (terminal table), `mermaid`, `json`, `plantuml`, `dsm`, `terminal`, `html`. |
| `--depth <D>` | _(auto)_ | Override the auto-detected directory depth used for grouping modules. Larger values produce finer-grained layers. |
| `--save` | `false` | Append the inferred layers to `.cha.toml` so `layer_violation` can enforce them. |

The global `--config <PATH>` flag also applies; `exclude` patterns are honoured.

## Workflow

1. `cha layers` — inspect what Cha thinks the layering looks like.
2. If it matches your intent, `cha layers --save`.
3. Add `enabled = true` under `[plugins.layer_violation]` if it isn't already.
4. Future `cha analyze` runs will fire an Error on any import that goes against the saved layering.

## See also

- [deps](./deps.md) — the underlying import graph.
- [`layer_violation` plugin](../plugins/reference.md)
- [Configuration overview](../configuration/overview.md)
