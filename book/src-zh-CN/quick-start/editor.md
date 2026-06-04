# 编辑器（LSP）

把 cha 内嵌进编辑器——保存时自动跑、下划线标 finding、悬停看说明。

## 两条路

**VS Code 用户**：直接装 [Cha 扩展](https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha)。扩展会自动下载匹配的 cha 二进制，不用手动装 cha。

**其他编辑器**：手动装 `cha`（[安装](../install.md)），然后让编辑器跑 `cha lsp` 作为 LSP 服务器。

## 编辑器里你会看到

- **诊断**——保存时跑 cha，finding 在文件里画下划线
- **悬停**——鼠标移到函数上出 markdown 报告卡：行数 / 圈复杂度 / 认知复杂度 / 参数数 / 这个函数的 finding 列表
- **CodeLens**——每个函数 / 类上方一行小字：有问题 `⚠ N issue(s)`，没问题 `✓ <行数>`
- **Inlay hint**——函数签名末尾一个小标记
- **Code action**——快速修复菜单（推荐 refactoring）+ Extract Method
- **大纲**（Document Symbol）—— 有问题的项前面加 `⚠`

## 下一步

- [LSP 详细能力清单](../lsp/overview.md) —— 11 个 LSP 请求各自做什么
- [VS Code 配置](../lsp/editors/vscode.md)
- [其他编辑器配置](../lsp/editors/others.md) —— Neovim / Helix / Zed / Sublime
