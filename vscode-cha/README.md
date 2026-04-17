# Cha - Code Smell Analyzer for VS Code

Real-time code smell detection and refactoring suggestions powered by [cha](https://github.com/W-Mai/Cha).

## Features

- 🔍 Real-time diagnostics — code smells highlighted as you type
- 💡 Code actions — refactoring suggestions via quick fix (Cmd+.)
- 🌐 Multi-language — Rust, TypeScript, Python, Go, C, C++

## Install

1. Download `.vsix` from [Releases](https://github.com/W-Mai/Cha/releases)
2. `code --install-extension vscode-cha-x.y.z.vsix`

Or search "Cha" in VS Code Marketplace (coming soon).

## Usage

Open any supported file — the extension starts automatically. If `cha` is not installed, it will offer to download it for you.

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `cha.path` | `"cha"` | Path to cha binary |
| `cha.lsp.enabled` | `true` | Enable/disable LSP |

## Development

```bash
cd vscode-cha
npm install
npm run compile
# F5 in VS Code to launch Extension Development Host
```

Package:
```bash
npx @vscode/vsce package --allow-missing-repository
```
