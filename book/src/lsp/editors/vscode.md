# VS Code

Install from the Marketplace:

**[Cha — Code Smell Analyzer](https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha)**

Or from the command line:

```bash
code --install-extension BenignX.vscode-cha
```

## First launch

On activation the extension checks for a `cha` binary on `PATH`. If it isn't found, the extension downloads the matching release for your platform from [github.com/W-Mai/Cha/releases](https://github.com/W-Mai/Cha/releases) and stores it under the extension's global storage directory. No manual install step is required.

The extension activates automatically when you open a file in any supported language: Rust, TypeScript / TSX, Python, Go, C, or C++.

## Configuration

The extension reads a `.cha.toml` in your workspace root (see [Configuration overview](../../configuration/overview.md)). No additional VS Code settings are needed for typical use.

Two settings exist if you need them:

| Setting | Default | Purpose |
|---------|---------|---------|
| `cha.path` | `"cha"` | Override the binary path. Set this to point at a development build or a non-`PATH` install. |
| `cha.lsp.enabled` | `true` | Disable the language client without uninstalling the extension. |

## Troubleshooting

- **No diagnostics appear.** Open the **Output** panel and select **Cha** from the dropdown. Failed downloads, missing binary, and LSP startup errors are logged there.
- **`cha` upgraded but the extension still uses the old version.** Reload the window (`Developer: Reload Window` from the command palette). The extension picks the binary path on activation.
- **Wrong binary picked up.** Set `cha.path` to an absolute path; the explicit setting overrides the auto-downloaded copy.
- **Binary download fails behind a corporate proxy.** Install `cha` manually (see [Installation](../../install.md)), make sure it's on `PATH`, and reload the window.

## What you get

Every capability listed in the [LSP overview](../overview.md): diagnostics, code actions (quick fixes + Extract Method), code lenses, hover cards, inlay hints, document symbols, semantic tokens, and workspace diagnostics. Nothing extra to configure.
