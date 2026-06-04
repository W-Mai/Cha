# Other editors

`cha lsp` is a standard LSP server over stdio. Any editor that supports custom language servers can use it. Snippets below are the minimum needed; consult each editor's documentation for full configuration options.

Register the server for these file types: `rust`, `typescript`, `typescriptreact`, `python`, `go`, `c`, `cpp`.

## Neovim (nvim-lspconfig)

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.cha then
  configs.cha = {
    default_config = {
      cmd = { 'cha', 'lsp' },
      filetypes = { 'rust', 'typescript', 'typescriptreact', 'python', 'go', 'c', 'cpp' },
      root_dir = lspconfig.util.root_pattern('.cha.toml', '.git'),
      single_file_support = false,
    },
  }
end

lspconfig.cha.setup {}
```

Run alongside your existing language server — Neovim merges diagnostics from multiple servers.

## Helix (`languages.toml`)

```toml
[language-server.cha]
command = "cha"
args = ["lsp"]

[[language]]
name = "rust"
language-servers = ["rust-analyzer", "cha"]

[[language]]
name = "typescript"
language-servers = ["typescript-language-server", "cha"]

# Repeat for: python, go, c, cpp; filetype name "tsx" covers .tsx files.
```

Place this in `~/.config/helix/languages.toml`.

## Zed

Zed uses extensions to declare custom language servers; the simplest path is a project-level config in `.zed/settings.json`:

```json
{
  "lsp": {
    "cha": {
      "binary": { "path": "cha", "arguments": ["lsp"] }
    }
  },
  "languages": {
    "Rust":       { "language_servers": ["rust-analyzer", "cha"] },
    "TypeScript": { "language_servers": ["typescript-language-server", "cha"] },
    "Python":     { "language_servers": ["pyright", "cha"] },
    "Go":         { "language_servers": ["gopls", "cha"] },
    "C":          { "language_servers": ["clangd", "cha"] },
    "C++":        { "language_servers": ["clangd", "cha"] }
  }
}
```

See the [Zed language-servers docs](https://zed.dev/docs/configuring-languages) for the canonical schema.

## Sublime Text (LSP plugin)

Install [LSP](https://packagecontrol.io/packages/LSP), then add this to `LSP.sublime-settings`:

```json
{
  "clients": {
    "cha": {
      "enabled": true,
      "command": ["cha", "lsp"],
      "selector": "source.rust | source.ts | source.tsx | source.python | source.go | source.c | source.c++"
    }
  }
}
```

See the [LSP plugin docs](https://lsp.sublimetext.io/client_configuration/) for selector syntax and per-window overrides.

## Verifying the setup

Open a file in a supported language and edit a function so it crosses a default threshold (e.g. 60 lines triggers `long_method` against the default of 50). On save you should see a `cha` diagnostic. If nothing appears, run `cha analyze` from the project root — if the CLI reports findings but the editor doesn't, the LSP client isn't routing diagnostics from the server.
