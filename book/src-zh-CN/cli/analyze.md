# analyze

跑一遍代码坏味道检测，是 Cha 用得最多的命令。

## 用法

```
cha analyze [参数] [路径...]
```

不给路径就扫当前目录（递归 + 遵循 `.gitignore`）。

## 示例

```bash
# 扫当前目录
cha analyze

# 指定路径 + JSON 输出，碰到 error 级 finding 就让 CI 失败
cha analyze src/ --format json --fail-on error

# 只扫工作区改动过的文件
cha analyze --diff

# 从管道读 diff（PR review 用）
gh pr diff | cha analyze --stdin-diff --fail-on warning

# 只跑指定插件
cha analyze --plugin complexity,naming

# 跳过缓存重跑
cha analyze --no-cache

# 拿 baseline 之外的新增 finding
cha analyze --baseline .cha/baseline.json

# 生成 HTML 报告
cha analyze --format html --output report.html
```

## 参数

| 参数 | 默认 | 说明 |
|------|------|------|
| `--format` | `terminal` | 输出格式：`terminal` / `json` / `llm` / `sarif` / `html` |
| `--fail-on` | — | finding 达到此严重度时退出码 1：`hint` / `warning` / `error` |
| `--diff` | `false` | 只扫工作区未提交的改动文件 |
| `--stdin-diff` | `false` | 从 stdin 读 unified diff，按 diff 里的范围扫 |
| `--plugin <名>` | 全开 | 只跑指定插件（逗号分隔） |
| `--no-cache` | `false` | 不用缓存（删掉再跑全量） |
| `--baseline <path>` | — | 只汇报不在 baseline 文件里的 finding |
| `--output <path>`, `-o` | — | 输出写到文件（HTML 等大体积格式用） |
| `--strictness <值>` | `default` | 阈值缩放：`relaxed`（2×）/ `default` / `strict`（0.5×）/ 自定义浮点 |
| `--all` | `false` | 终端格式：所有 finding 全列，不聚合 |
| `--top <N>` | — | 终端格式：只显示前 N 条最严重的 |
| `--focus <类目>` | — | 只看指定类目（逗号分隔）：`bloaters` / `oo_abusers` / `change_preventers` / `dispensables` / `couplers` / `security` |

JSON 格式的 schema 用 [`cha schema`](./init.md) 拿。

## 参考

- [输出格式总览](../output/index.md)
- [配置](../configuration/overview.md)
- [`cha baseline`](./baseline.md) —— 生成 baseline 文件
- [`cha fix`](./fix.md) —— 自动修一部分 finding
