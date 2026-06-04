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

Three settings exist if you need them:

| Setting | Default | Purpose |
|---------|---------|---------|
| `cha.path` | `"cha"` | Override the binary path. Set this to point at a development build or a non-`PATH` install. |
| `cha.lsp.enabled` | `true` | Disable the language client without uninstalling the extension. |
| `cha.disabledPlugins` | `["large_file"]` | Smell names the extension should suppress in-editor. `large_file` is off by default — when you're already inside a file, "this file is too long" is noise. Add `todo_comment`, `high_coupling`, etc. if you find them distracting at edit time; they still surface in `cha analyze` from the CLI. |

## Troubleshooting

- **No diagnostics appear.** Open the **Output** panel and select **Cha** from the dropdown. Failed downloads, missing binary, and LSP startup errors are logged there.
- **`cha` upgraded but the extension still uses the old version.** Reload the window (`Developer: Reload Window` from the command palette). The extension picks the binary path on activation.
- **Wrong binary picked up.** Set `cha.path` to an absolute path; the explicit setting overrides the auto-downloaded copy.
- **Binary download fails behind a corporate proxy.** Install `cha` manually (see [Installation](../../install.md)), make sure it's on `PATH`, and reload the window.

## What you get

Every capability listed in the [LSP overview](../overview.md). What that means in practice inside VS Code:

| Capability | What it shows up as |
|---|---|
| Diagnostics | Wavy underlines + entries in the **Problems** panel. Severity follows the smell's severity (`Error` / `Warning` / `Information`). |
| Code actions | Lightbulb on the offending line; `Cmd+.` (or `Ctrl+.`) to apply. Includes Fowler-style refactorings the analyzer suggests, plus the built-in **Extract Method** for over-long functions. |
| Code lens | Inline overlays above functions / classes — complexity, line count, parameter count. |
| Inlay hints | Subtle gray annotations after function names: `cx:N cog:N NL` (cyclomatic / cognitive / nesting). Toggle via the **Editor › Inlay Hints** setting if they get noisy. |
| Hover | Hover any function or class to see a markdown "report card" — its metrics and which thresholds it crosses. |
| Document symbols | Outline view (`⌘⇧O`) with ⚠ markers on items that have findings. |
| Semantic tokens | Functions / classes with findings get a `warning` modifier — useful with themes that style it (e.g. yellow underline). |
| Workspace diagnostics | `cha analyze` against the whole project on extension activation, so the **Problems** panel populates without opening every file first. Progress shows in the status bar. |

No extra configuration needed for any of the above.

## Marketplace

Listing: <https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha>

Released alongside `cha` itself — install the latest extension and you get the latest analyzer.
