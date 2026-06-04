# init / schema

Two small bootstrap commands. `init` writes a starter config; `schema` prints the
JSON Schema for findings so external tools can validate `--format json` output.

## init

Write a default `.cha.toml` to the current directory.

### Usage

```
cha init
```

### Example

```bash
cha init
# → Created .cha.toml
```

If `.cha.toml` already exists the command exits with status 1 and prints
`.cha.toml already exists` — it never overwrites.

The template lives at [`static/default.cha.toml`](https://github.com/W-Mai/Cha/blob/main/static/default.cha.toml)
in the repository and is embedded in the binary at compile time.

### Flags

`init` takes no flags.

## schema

Print the JSON Schema for the analysis output format on stdout.

### Usage

```
cha schema
```

### Examples

```bash
# Save the schema for editor / CI integrations.
cha schema > findings.schema.json

# Validate a previous run against the schema (uses ajv-cli for example).
cha analyze --format json > findings.json
ajv validate -s findings.schema.json -d findings.json
```

The schema describes the same structure produced by `cha analyze --format json`;
see also the dedicated [JSON Schema](../reference/json-schema.md) reference page.

### Flags

`schema` takes no flags.

## See also

- [Configuration overview](../configuration/overview.md) — every key supported by
  `.cha.toml`.
- [JSON output](../output/json.md) — the format the schema describes.
- [JSON Schema reference](../reference/json-schema.md).
