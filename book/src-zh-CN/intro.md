# 简介

<p align="center">
  <img src="https://raw.githubusercontent.com/W-Mai/Cha/main/static/logo.svg" alt="cha logo" width="160"/>
</p>

<p align="center">
  <strong>察 — 代码健康度分析器</strong>
</p>

<p align="center">
  <a href="https://github.com/W-Mai/Cha/actions"><img src="https://img.shields.io/github/actions/workflow/status/W-Mai/Cha/ci.yml?style=flat-square" alt="CI" /></a>
  <a href="https://github.com/W-Mai/Cha/blob/main/LICENSE"><img src="https://img.shields.io/github/license/W-Mai/Cha?style=flat-square" alt="License" /></a>
  <a href="https://github.com/W-Mai/Cha"><img src="https://img.shields.io/github/stars/W-Mai/Cha?style=flat-square" alt="Stars" /></a>
  <a href="https://github.com/W-Mai/Cha/releases"><img src="https://img.shields.io/github/v/release/W-Mai/Cha?style=flat-square" alt="Release" /></a>
</p>

**Cha**（察，「审视、查看」）是一个可插拔的代码坏味道检测工具集。它通过 tree-sitter 在 AST 层解析源码，运行 34 个内置检测器以及用户提供的 WASM 插件，并以终端输出、JSON、LLM 上下文、SARIF 或 HTML 形式呈现结果。

支持语言：Python（`.py`）、TypeScript / TSX（`.ts`、`.tsx`、`.mts`、`.cts`）、Rust（`.rs`）、Go（`.go`）、C（`.c`、`.h`）、C++（`.cpp`、`.cc`、`.cxx`、`.hpp`、`.hxx`）。

## 文档结构

- **[安装](./install.md)** —— 把 `cha` 装到你机器上
- **[快速开始](./quick-start/cli.md)** —— 五分钟跑通几种典型用法
- **[配置](./configuration/overview.md)** —— `.cha.toml`、严格度、行内指令
- **[Smell 列表](./plugins/reference.md)** —— 每一条内置检测器、触发条件、调参方法
- **[插件开发](./plugins/development.md)** —— 自己写 WASM 插件
- **[命令行](./cli/index.md)** —— 每个子命令的细节
- **[LSP 集成](./lsp/overview.md)** —— 接到你常用的编辑器
- **[Cookbook](./recipes/index.md)** —— 按场景的菜谱

## 状态

Cha 处于 1.0 之前——核心引擎稳定且自检测，但配置格式和命令行接口仍在演进。每次破坏性变更都会写进 CHANGELOG。
