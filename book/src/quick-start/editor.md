# Quick Start: Editor

Cha ships an LSP server, so any LSP-capable editor can show findings inline.

## VS Code

Install the [Cha extension](https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha) from the Marketplace. The extension auto-downloads a matching `cha` binary on first launch — you don't need to install the CLI separately.

That's the whole setup. See [LSP / VS Code](../lsp/editors/vscode.md) for the extension's settings.

## Other editors

Any editor that speaks LSP (Neovim, Helix, Zed, Sublime, Emacs with `lsp-mode`, …) can talk to Cha. Install the CLI ([Quick Start: CLI](./cli.md) step 1) and point your editor at:

```
cha lsp
```

as the language server command. Per-editor config recipes live in [LSP / Other editors](../lsp/editors/others.md).

## What you'll see

Once the LSP is connected, opening a source file gives you:

- **Diagnostics** — every finding becomes a squiggle under the offending line, with severity matching `hint` / `warning` / `error`.
- **CodeLens** — small annotations above each function and class showing complexity, line count, and parameter count.
- **Hover** — hovering over a flagged symbol shows a markdown report card (which detector fired, what threshold, suggested refactorings).
- **Inlay hints** — inline `cx:N cog:N NL` markers next to function names.
- **Code actions** — quick-fix entries for refactorings the detector can suggest, including Extract Method.
- **Document symbols** — the outline view marks problematic items with ⚠.
- **Workspace diagnostics** — the server can scan the whole project without you opening each file.

Full capability list and what each one looks like in practice: [LSP overview](../lsp/overview.md).
