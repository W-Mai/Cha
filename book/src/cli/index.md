# CLI

Cha exposes 15 top-level subcommands plus two nested groups (`plugin` / `preset`).

A global `--config <path>` flag overrides the default `.cha.toml` lookup for any
subcommand. Run `cha <command> --help` for inline reference, or `cha help-markdown`
to dump the full clap manual.

## Analysis

| Command | Purpose |
|---------|---------|
| [`analyze`](./analyze.md) | Run all enabled plugins over the given paths and report findings. |
| [`parse`](./parse.md) | Print the parsed structure (functions, classes, imports) of each file. |
| [`fix`](./fix.md) | Apply automatic fixes for plugins that provide them (currently naming). |
| [`baseline`](./baseline.md) | Snapshot current findings to a baseline file so future runs only flag new issues. |

## Architecture and history

| Command | Purpose |
|---------|---------|
| [`deps`](./deps.md) | Render import / class / call graphs (DOT, JSON, Mermaid, PlantUML, DSM, terminal, HTML). |
| [`layers`](./layers.md) | Infer architectural layers from import dependencies. |
| [`hotspot`](./hotspot.md) | Rank files by `change frequency × complexity` from `git log`. |
| [`trend`](./trend.md) | Analyze recent commits and chart the issue count over time. |

## Configuration and tuning

| Command | Purpose |
|---------|---------|
| [`init`](./init.md) | Write a default `.cha.toml` to the current directory. |
| [`schema`](./init.md#schema) | Print the JSON Schema for the analysis output format. |
| [`calibrate`](./calibrate.md) | Suggest thresholds from project statistics (P90 = warning, P95 = error). |
| [`preset`](./preset.md) | List builtin language profiles or show one in detail. |

## Plugin lifecycle

The `plugin` group wraps the WASM plugin workflow. See [`plugin`](./plugin.md) for the
subcommand list, and [Plugin development](../plugins/development.md) for the full guide.

| Command | Purpose |
|---------|---------|
| `plugin new <name>` | Scaffold a new plugin crate. |
| `plugin build` | Build the current crate as a WASM component. |
| `plugin install <path>` | Copy a `.wasm` file into `.cha/plugins/`. |
| `plugin list` | List installed plugins (local and global). |
| `plugin remove <name>` | Delete an installed plugin. |

## Editor integration

| Command | Purpose |
|---------|---------|
| `lsp` | Start the Language Server Protocol server. See [LSP integration](../lsp/overview.md). |
| [`completions`](./completions.md) | Emit shell completion scripts (bash, zsh, fish, powershell, elvish), with dynamic plugin name completion. |

## Hidden

`help-markdown` dumps the full clap manual as markdown. It is hidden from `--help`
output and intended for documentation generation.
