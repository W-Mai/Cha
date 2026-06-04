# LSP 概览

启动语言服务器：

```
cha lsp
```

服务器跑在 stdio 上，VS Code 扩展自动启动。其他编辑器见 [其他编辑器](editors/others.md)。

## 实现的能力

实际代码在 [`cha-lsp/src/lib.rs`](https://github.com/W-Mai/Cha/blob/main/cha-lsp/src/lib.rs)。

### 生命周期

- **`initialize`** —— 申明服务器能力。读 `initializationOptions.disabledPlugins`（JSON 数组），编辑器可以按工作区禁用某些插件
- **`initialized`** —— 后台跑一遍全工作区分析：从根目录走 `.gitignore` 过滤后的所有源文件，解析 + 缓存
- **`shutdown`** —— 干净退出

### 文档同步

- **`textDocument/didOpen`** —— 缓存文档文本，给后续 code action 和 inlay hint 用
- **`textDocument/didChange`** —— 全文档同步（`TextDocumentSyncKind::FULL`）。**只更新内存里的文本，不重新分析**
- **`textDocument/didSave`** —— 触发全工作区重新分析，刷新所有 finding 和缓存

### 诊断

- **`textDocument/diagnostic`** —— 单文件 pull 模式诊断。每个 finding 变成一条 `Diagnostic`，`source = "cha"`，`code` 是 smell 名，severity 映射 hint/warning/error
- **`workspace/diagnostic`** —— 整个项目的 pull diagnostics。让编辑器 Problems 面板不用打开每个文件就能填满

### 代码智能

- **`textDocument/codeAction`** —— 两类：
  1. **Quick fix** ——任何带建议的 cha 诊断都会出 `Refactor: <建议>` 选项
  2. **Extract Method** —— `long_method` 诊断、或者用户手选 ≥3 行时出现，生成 `WorkspaceEdit` 把选中段抽成 `extracted()` 函数
- **`textDocument/codeLens`** —— 每个函数 / 类上方一条 lens：有问题时 `⚠ N issue(s) | <行数>`，没问题时 `✓ <行数>`
- **`textDocument/hover`** —— 函数 hover 出 markdown 报告卡：名字、行数、圈复杂度、认知复杂度、参数数、链深；下面是这函数的所有 finding
- **`textDocument/inlayHint`** —— 函数签名末尾一个小标记：有问题 `⚠N`，没问题 `✓`
- **`textDocument/documentSymbol`** —— 大纲视图（嵌套）。函数显示 `cx:<复杂度> <行数>L`，类显示 `<方法数>m <字段数>f <行数>L`。有 warning / error 的项前面加 `⚠`
- **`textDocument/semanticTokens/full`** —— 暴露 `function` / `class` 两种 token 类型 + 一个 `warning` modifier。支持 semantic token 主题的编辑器能给有问题的项加高亮

## 没实现的

下面这些标准 LSP 请求 `cha lsp` 不提供：

- `textDocument/completion`
- `textDocument/definition`
- `textDocument/references`
- `textDocument/rename`
- `textDocument/signatureHelp`
- `textDocument/formatting`
- `workspace/didChangeConfiguration`

这些通常由语言自己的 LSP 服务器（rust-analyzer / pyright / gopls 等）提供。`cha lsp` 跟语言 LSP 一起跑就行——多数编辑器会合并多个 server 的结果。

## 重新分析的触发点

全工作区重跑只发生在 `didSave`，不是每次按键。所以编辑过程中的诊断**保存前是滞后**的——这是有意设计，避免按一下键就跑一遍千文件分析。on-disk 缓存（`.cha/cache/`）让 warm 跑通常亚秒级。
