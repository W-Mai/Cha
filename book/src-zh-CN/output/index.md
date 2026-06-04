# 输出格式

`cha analyze --format <格式>` 选输出。5 种格式各自针对不同消费者：

| 格式 | 适用 | CLI 参数 |
|------|------|---------|
| [`terminal`](./terminal.md) | 人眼读，本地开发默认 | `--format terminal`（默认） |
| [`json`](./json.md) | 工具消费、CI 后处理、jq | `--format json` |
| [`sarif`](./sarif.md) | GitHub Code Scanning、安全平台 | `--format sarif` |
| [`html`](./html.md) | 静态报告分发、邮件附件 | `--format html --output report.html` |
| [`llm`](./llm.md) | 喂给 LLM 当上下文 | `--format llm` |

JSON 的字段定义跟 [JSON Schema 参考](../reference/json-schema.md) 对齐——schema 用 `cha schema` 拿。

## 通用参数

下面这几个参数对所有格式（或大多数格式）有效：

- `--output <path>`, `-o`：写到文件而不是 stdout（HTML 这种大体积的强烈推荐）
- `--fail-on <级别>`：finding 达到 hint / warning / error 时退出码 1
- `--top <N>`：只看前 N 条最严重的（终端格式）
- `--all`：终端格式不聚合，全部展开
- `--focus <类目>`：只看指定类目（bloaters / oo_abusers / change_preventers / dispensables / couplers / security）
