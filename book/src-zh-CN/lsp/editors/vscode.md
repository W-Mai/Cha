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

如果需要可以改两个：

| 设置 | 默认 | 用途 |
|------|------|------|
| `cha.path` | `"cha"` | 改二进制路径——指向开发版或非 PATH 安装 |
| `cha.lsp.enabled` | `true` | 不卸载扩展，只关 LSP 客户端 |

## 故障排除

- **看不到诊断** —— 打开 **Output** 面板下拉选 **Cha**，下载失败 / 二进制找不到 / LSP 启动错误都在那
- **cha 升级了但扩展用旧版** —— Reload window（命令面板 `Developer: Reload Window`）。扩展激活时锁定二进制路径
- **想用别的二进制** —— `cha.path` 设绝对路径覆盖
- **公司代理下不下来** —— 手动 [装 cha](../../install.md)，确保在 PATH 上，再 reload window

## 你能用的能力

[LSP 概览](../overview.md) 列的全套：诊断、code action（quick fix + Extract Method）、code lens、hover、inlay hint、document symbol、semantic tokens、workspace diagnostics。VS Code 端不用配额外设置。
