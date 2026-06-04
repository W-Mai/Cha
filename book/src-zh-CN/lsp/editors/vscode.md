# VS Code

直接 Marketplace 装：

**[Cha — Code Smell Analyzer](https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha)**

或者命令行：

```bash
code --install-extension BenignX.vscode-cha
```

## 第一次启动

扩展激活时检查 `PATH` 上有没有 `cha` 二进制。没有就**自动从 GitHub Releases 下载**对应平台的 cha，存到扩展全局存储目录。**不用手动装 cha**。

打开 Rust / TypeScript / TSX / Python / Go / C / C++ 文件时自动激活。

## 配置

主要靠工作区根目录的 `.cha.toml`（见 [配置概览](../../configuration/overview.md)）。VS Code 端基本不用配。

如果需要可以改三个：

| 设置 | 默认 | 用途 |
|------|------|------|
| `cha.path` | `"cha"` | 改二进制路径——指向开发版或非 PATH 安装 |
| `cha.lsp.enabled` | `true` | 不卸载扩展，只关 LSP 客户端 |
| `cha.disabledPlugins` | `["large_file"]` | 编辑器里要屏蔽的 smell 名。`large_file` 默认就关着——人都进文件编辑了，再提示"这个文件太长"是噪音。觉得 `todo_comment`、`high_coupling` 等编辑时碍眼也可以加进来；CLI 跑 `cha analyze` 时这些 smell 仍然会报。 |

## 故障排除

- **看不到诊断** —— 打开 **Output** 面板下拉选 **Cha**，下载失败 / 二进制找不到 / LSP 启动错误都在那
- **cha 升级了但扩展用旧版** —— Reload window（命令面板 `Developer: Reload Window`）。扩展激活时锁定二进制路径
- **想用别的二进制** —— `cha.path` 设绝对路径覆盖
- **公司代理下不下来** —— 手动 [装 cha](../../install.md)，确保在 PATH 上，再 reload window

## 你能用的能力

[LSP 概览](../overview.md) 列的全套，落到 VS Code 里的具体形态：

| 能力 | 在 VS Code 里看到什么 |
|---|---|
| 诊断 | 代码下面的波浪线 + **Problems** 面板里的条目。严重度跟着 smell 走（`Error` / `Warning` / `Information`）。 |
| Code action | 出问题的那行旁边出小灯泡，`Cmd+.`（Mac）或 `Ctrl+.` 触发。除了 Fowler 风格的重构建议，超长函数还能用内置的 **Extract Method**。 |
| Code lens | 函数 / 类上方一行小字标注——复杂度、行数、参数数量。 |
| Inlay hint | 函数名后面跟一截灰色 `cx:N cog:N NL`（圈复杂度 / 认知复杂度 / 嵌套层数）。觉得碍眼可以在 **Editor › Inlay Hints** 设置里关掉。 |
| Hover | 鼠标悬停在函数或类上，弹一份 markdown "评分卡"——指标值、越过哪些阈值。 |
| Document symbol | Outline 视图（`⌘⇧O`），有 finding 的 item 前面带 ⚠ 标记。 |
| Semantic tokens | 出问题的函数 / 类带 `warning` modifier，配支持这个 modifier 的主题（比如黄色下划线）效果更明显。 |
| Workspace diagnostics | 扩展激活时 `cha analyze` 整个项目，不用挨个打开文件 **Problems** 也能填满。进度在状态栏。 |

上面这些都不需要在 VS Code 端额外配什么。

## Marketplace

Listing：<https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha>

跟 `cha` 一起发布——装最新扩展就能用上最新的分析器。
