# Overview

Run the language server with:

```
cha lsp
```

The server speaks LSP over stdio and is registered automatically by the VS Code extension. For other editors, see [Other editors](editors/others.md).

## Capabilities

The server registers the following capabilities at `initialize` time and implements the matching request handlers in [`cha-lsp/src/lib.rs`](https://github.com/W-Mai/Cha/blob/main/cha-lsp/src/lib.rs).

### Lifecycle

- **`initialize`** — declares server capabilities. Reads `initializationOptions.disabledPlugins` (a JSON array of plugin names) so editors can suppress specific detectors per-workspace.
- **`initialized`** — runs a full workspace analyze in the background: walks the project root with `.gitignore` honoured, parses every `.rs / .ts / .tsx / .py / .go / .c / .h / .cpp / .cc / .cxx / .hpp`, and warms the project cache.
- **`shutdown`** — clean exit, no extra teardown.

### Document sync

- **`textDocument/didOpen`** — caches the document text in memory so code actions and inlay hints can read selection content.
- **`textDocument/didChange`** — full-document sync (`TextDocumentSyncKind::FULL`); updates the in-memory text but does not re-analyze (re-analysis happens on save).
- **`textDocument/didSave`** — re-runs full workspace analyze. Findings, parsed models, and the on-disk cache are refreshed.

### Diagnostics

- **`textDocument/diagnostic`** — pull-based per-file diagnostics. Each finding becomes an LSP `Diagnostic` with `source = "cha"`, code set to the smell name, and severity mapped from `Hint / Warning / Error`. If the finding has refactoring suggestions, they ride along in the diagnostic's `data` field for code actions to pick up.
- **`workspace/diagnostic`** — full project diagnostics in one report. Lets the editor populate the Problems panel without opening every file.

### Code intelligence

- **`textDocument/codeAction`** — surfaces two kinds of refactorings:
  1. **Quick fixes** for any cha diagnostic with attached suggestions (`Refactor: <suggestion>`).
  2. **Extract Method** — offered for `long_method` diagnostics, and also when the user selects 3+ lines manually. Generates a `WorkspaceEdit` that replaces the selection with a call to a new `extracted()` function appended below.
- **`textDocument/codeLens`** — one lens above every function and class. Shows `⚠ N issue(s) | <lines>` when the item has findings, otherwise `✓ <lines>` (functions) or `✓ <methods>m <fields>f <lines>L` (classes).
- **`textDocument/hover`** — markdown report card on hover over a function: name, lines, cyclomatic complexity, cognitive complexity, parameter count, chain depth, and a bullet list of findings with severity icons.
- **`textDocument/inlayHint`** — end-of-signature hint per function: `⚠N` if the function has findings, otherwise `✓`.
- **`textDocument/documentSymbol`** — outline view (nested). Functions show `cx:<complexity> <lines>L`, classes show `<methods>m <fields>f <lines>L`. Items containing warning- or error-level findings are prefixed with `⚠`.
- **`textDocument/semanticTokens/full`** — exposes two token types (`function`, `class`) with one modifier (`warning`). Editors with semantic-token theming can highlight items that have findings.

## Not implemented

The following standard LSP requests are not provided by `cha lsp`:

- `textDocument/completion`
- `textDocument/definition`
- `textDocument/references`
- `textDocument/rename`
- `textDocument/signatureHelp`
- `textDocument/formatting`
- `workspace/didChangeConfiguration`

These are usually supplied by the language's own LSP server (rust-analyzer, pyright, gopls, …). Run `cha lsp` alongside the language LSP — most editors merge results from multiple servers.

## Re-analysis trigger

The full workspace re-analyzes on `didSave`, not on every keystroke. Diagnostics for in-progress edits stay stale until you save. The on-disk cache (`.cha/cache/`) means warm runs are typically sub-second.
