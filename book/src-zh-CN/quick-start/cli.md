# 命令行

5 分钟从 0 到第一个 finding。

## 1. 装

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.sh | sh

# Windows
powershell -c "irm https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.ps1 | iex"

# Homebrew
brew install W-Mai/cellar/cha-cli
```

详见 [安装](../install.md)。

## 2. 第一次跑

进项目目录：

```bash
cd path/to/your-repo
cha analyze
```

输出大致长这样：

```
ℹ [data_class] src/types.rs:8-15 Class `User` has 4 fields but no behavior methods, consider Move Method
ℹ [long_method] src/handlers.rs:42 Function `process` is 78 lines (threshold: 50)
⚠ [high_complexity] src/parser.rs:120 Function `parse` has cyclomatic complexity 14 (threshold: 10)
…

47 issue(s) found (0 error, 3 warning, 44 hint).
Tech debt: ~3h 25min | A:12 B:5 C:1 D:0 F:0
```

每行：严重度图标 + smell 名 + 位置 + 一句话原因。底部有按严重度统计 + 估算技术债。

## 3. 看详细的

某条 smell 不熟？点 [Smell 列表](../plugins/reference.md) 去翻——34 个内置检测器都有"在抓什么、阈值含义、触发例子"。

或者拿 JSON 给工具消费：

```bash
cha analyze --format json | jq '.findings | group_by(.smell_name) | map({smell: .[0].smell_name, count: length})'
```

## 4. 调严或调宽

不爽默认阈值？写 `.cha.toml`：

```bash
cha init
```

生成的 `.cha.toml` 已经带常用插件的默认阈值注释，改数字就行：

```toml
[plugins.length]
max_function_lines = 80   # 我们的代码风格函数普遍偏长，把上限抬到 80

[plugins.complexity]
warn_threshold = 15
error_threshold = 25
```

或者用全局 strictness 缩放：

```toml
strictness = "relaxed"  # 所有数值阈值翻倍
```

详见 [配置概览](../configuration/overview.md) 和 [严格度与预设](../configuration/presets.md)。

## 5. 接老项目

老仓库一上来几百条 finding 没法治？拿一份 baseline，新增的才报：

```bash
# 拍快照
cha baseline                                  # 写到 .cha/baseline.json

# 后续只看 baseline 之外的新问题
cha analyze --baseline .cha/baseline.json --fail-on warning
```

详见 [`cha baseline`](../cli/baseline.md) 和 [Baseline 工作流](../recipes/baseline.md)。

## 接下来

- [pre-commit hook](pre-commit.md) —— 提交前自动跑
- [GitHub Actions](github-actions.md) —— PR 自动检查
- [编辑器（LSP）](editor.md) —— VS Code / Neovim / Helix 等内嵌
- [Cookbook](../recipes/index.md) —— 常见场景菜谱
