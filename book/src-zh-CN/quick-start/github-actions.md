# GitHub Actions

PR 一开就跑 cha，把 finding 自动喷到 Code Scanning 里——你在 PR 文件 diff 旁边能直接看到 cha 标的问题。

## 最简单的：跑 + 上传 SARIF

`.github/workflows/cha.yml`：

```yaml
name: Cha
on:
  push:
    branches: [main]
  pull_request:
permissions:
  contents: read
  security-events: write
jobs:
  cha:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: W-Mai/Cha@v1.19.0
        with:
          fail-on: warning
          upload-sarif: true
```

`upload-sarif: true` 把结果直接传给 GitHub Code Scanning（要 `security-events: write` 权限）。之后 PR diff 旁边、Security 标签下都能看到 finding。

## Action 的输入

| 输入 | 默认 | 说明 |
|------|------|------|
| `version` | `latest` | 装哪个 cha 版本 |
| `format` | `sarif` | 输出格式：`terminal` / `json` / `sarif` / `html` |
| `fail-on` | `error` | 报到这个严重度就让 job 失败：`hint` / `warning` / `error` |
| `plugin` | — | 只跑指定插件（逗号分隔） |
| `paths` | `.` | 扫哪些路径 |
| `upload-sarif` | `false` | 是否自动上传 SARIF 到 Code Scanning（要 `security-events: write`） |

## 进阶：JSON + 自己处理

不想用 Code Scanning，想自己后处理：

```yaml
- uses: W-Mai/Cha@v1.19.0
  with:
    format: json
    fail-on: warning
- run: |
    cha analyze --format json > findings.json
    # 用 jq 过滤、丢给内部 dashboard、贴到 PR 评论 …
```

## 进阶：只跑改动的文件（PR）

PR 上只检查这次改的，不扫全仓：

```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 0  # 需要历史才能算 diff
- uses: W-Mai/Cha@v1.19.0
  with:
    paths: ''       # 留空让 action 用 --diff
    fail-on: warning
```

或者直接手动调：

```yaml
- run: cha analyze --diff --fail-on warning --format sarif --output cha.sarif
- uses: github/codeql-action/upload-sarif@v3
  if: always()
  with:
    sarif_file: cha.sarif
```

`if: always()` 让 SARIF 上传步骤即使前一步 `--fail-on` 失败了也跑——不然 finding 反而上传不了。

## 接下来

- [SARIF 输出格式](../output/sarif.md) —— Code Scanning 集成的细节
- [pre-commit hook](pre-commit.md) —— 提交前本地兜底
- [Monorepo CI](../recipes/monorepo-ci.md) —— 多个子项目的 CI 配置
