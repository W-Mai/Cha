# 命令行

Cha 一共 15 个顶层子命令，加上 `plugin` 和 `preset` 两个嵌套组。下面按用途分。

完整 `--help` 树用 [`cha help-markdown`](./completions.md) 可以一次拉全。

## 分析

| 命令 | 用途 |
|------|------|
| [`analyze`](./analyze.md) | 跑插件，报代码坏味道——最常用 |
| [`parse`](./parse.md) | dump 解析结果（函数 / 类 / import / 注释） |

## 报告与历史

| 命令 | 用途 |
|------|------|
| [`baseline`](./baseline.md) | 把当前 finding 拍快照，老问题屏蔽 |
| [`trend`](./trend.md) | 看 finding 数随 commit 怎么变 |
| [`hotspot`](./hotspot.md) | 改动频度 × 复杂度的热点 |
| [`deps`](./deps.md) | 依赖图：import / 类 / 调用 |
| [`layers`](./layers.md) | 从 import 反推架构层级 |

## 配置与调优

| 命令 | 用途 |
|------|------|
| [`init`](./init.md) | 生成默认 `.cha.toml` |
| `schema` | 打印 finding JSON Schema（细节同 init 页） |
| [`calibrate`](./calibrate.md) | 按项目统计推荐阈值（P90 / P95） |
| [`preset`](./preset.md) | 看内置语言 profile 和严格度等级 |

## 自动修复

| 命令 | 用途 |
|------|------|
| [`fix`](./fix.md) | 自动改简单问题（目前只支持 `naming_convention`） |

## 插件

| 命令 | 用途 |
|------|------|
| [`plugin new`](./plugin.md) | 脚手架一个 WASM 插件 |
| [`plugin build`](./plugin.md) | 编译 + 打包成 WASM Component |
| [`plugin install`](./plugin.md) | 装到 `.cha/plugins/` |
| [`plugin list`](./plugin.md) | 列已装插件 |
| [`plugin remove`](./plugin.md) | 删插件 |

[插件开发指南](../plugins/development.md) 有完整流程。

## 编辑器集成

| 命令 | 用途 |
|------|------|
| [`lsp`](../lsp/overview.md) | 启 LSP 服务器（标准 stdio 协议） |
| [`completions`](./completions.md) | 生成 shell 补全脚本 |
