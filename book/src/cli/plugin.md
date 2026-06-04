# plugin

Lifecycle commands for WASM plugins. Plugins are WebAssembly Component Model modules
loaded from `.cha/plugins/` (project-local) or `~/.cha/plugins/` (global).

This page is a thin reference for the five subcommands. The end-to-end workflow —
project layout, the SDK, the WIT interface, debugging, publishing — is in
[Plugin development](../plugins/development.md).

## Usage

```
cha plugin new <name>
cha plugin build
cha plugin install <path>
cha plugin list
cha plugin remove <name>
```

## Examples

```bash
# Scaffold a new plugin in ./my-plugin/ (or in cwd if cwd is empty).
cha plugin new my-plugin

# Build the crate in cwd to a WASM component (`<package>.wasm`).
cd my-plugin && cha plugin build

# Install a built artefact into .cha/plugins/.
cha plugin install my_plugin.wasm

# List installed plugins, with version, description, authors, and emitted smells.
cha plugin list

# Remove a plugin (extension optional).
cha plugin remove my_plugin
```

## Subcommands

### `new <name>`

Creates `Cargo.toml` and `src/lib.rs` from the SDK template. If cwd is empty it
scaffolds in place; otherwise it creates a subdirectory named `<name>`.

### `build`

Runs `cargo build --target wasm32-wasip1 --release` and converts the resulting core
module to a Component Model artefact via `wit-component`. The output filename is
the package name (with `-` replaced by `_`) plus `.wasm`.

### `install <path>`

Copies the `.wasm` file into `.cha/plugins/` in the current project. Creates the
directory if it doesn't exist.

### `list`

Shows installed plugins for both `.cha/plugins/` (local) and `~/.cha/plugins/`
(global). For each plugin the version, description, authors, and emitted smells are
read directly from the WASM artefact.

### `remove <name>`

Deletes a plugin from `.cha/plugins/`. The `.wasm` extension is optional.

## See also

- [Plugin development](../plugins/development.md) — full guide.
- [`analyze --plugin`](./analyze.md) — run only specific plugins (supports dynamic
  completion of installed plugin names).
- [`completions`](./completions.md) — enables tab-completion for plugin names.
