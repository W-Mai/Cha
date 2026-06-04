# JSON Schema

`cha schema` 输出一份 [JSON Schema 2020-12](https://json-schema.org/draft/2020-12/release-notes) 文档，描述的是 `cha analyze --format json` 的输出结构。可以用它来校验 cha 的输出、给其他语言生成类型、给读取 cha findings 的下游工具配 IDE 自动补全。

这**不是** `.cha.toml` 的 schema —— cha 的配置没有发布 schema。配置 key 看 [配置项参考](./config-keys.md)。

## 生成

```bash
cha schema > cha-findings.schema.json
```

输出是 `Vec<Finding>` 的 schema，从 [`cha-core/src/model.rs`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/model.rs) 里的 `Finding` 结构体通过 [`schemars`](https://crates.io/crates/schemars) 自动派生而来。每次发版都会重新派生一次，schema 跟当前 `Finding` 结构始终一致。

## 怎么用

### 校验 JSON 输出

```bash
cha analyze --format json > findings.json
cha schema > cha-findings.schema.json

# 选一个 JSON Schema 校验器；这里举 check-jsonschema 为例：
check-jsonschema --schemafile cha-findings.schema.json findings.json
```

退出码 0 表示输出符合 schema。如果有下游工具依赖 cha 的 JSON 输出格式，把这一步加进 CI 能在 cha 升级导致 schema 变化时提前发现，避免下游静默坏掉。

### 给 IDE 配自动补全

如果你写工具或脚本直接读 `findings.json`，把 schema 喂给编辑器的 JSON 支持就行。VS Code：

```jsonc
// .vscode/settings.json
{
  "json.schemas": [
    {
      "fileMatch": ["**/findings.json"],
      "url": "./cha-findings.schema.json"
    }
  ]
}
```

走 [`schemastore.org`](https://schemastore.org) 的编辑器（Helix、用 `efm-langserver` 的 Neovim 等）可以自己加映射。我们暂时没把 schema 提交到 schemastore，所以路径只能是本地的。

### 给其他语言生成类型

[`quicktype`](https://quicktype.io) 吃 JSON Schema、产出 TypeScript / Python / Java / C# / Go / Rust 等：

```bash
quicktype --src-lang schema cha-findings.schema.json -o ChaFindings.ts
```

输出是带类型的 dataclass / interface，跟 `Finding` 结构对应。写 dashboard、exporter 或 LSP 旁边的工具时这种生成挺有用。

## 一条 Finding 长什么样

schema 描述的就是这种东西，每条分析结果一个：

```json
{
  "smell_name": "long_method",
  "category": "Bloaters",
  "severity": "Warning",
  "location": {
    "path": "src/handlers.rs",
    "start_line": 142,
    "start_col": 8,
    "end_line": 198,
    "end_col": 1,
    "name": "process_request"
  },
  "message": "Function `process_request` is 87 lines (threshold: 50)",
  "suggested_refactorings": ["Extract Method"],
  "actual_value": 87.0,
  "threshold": 50.0,
  "risk_score": 1.74
}
```

| 字段 | 含义 |
|---|---|
| `smell_name` | smell ID，如 `long_method`。多个插件能不能共用同一个 smell 名取决于它们配合，目前没有这种配置。 |
| `category` | 取值 `Bloaters` / `Couplers` / `OOAbusers` / `ChangePreventers` / `Dispensables` / `Security` 之一。驱动 `--focus` 和输出分组。 |
| `severity` | `Hint` / `Warning` / `Error`。驱动 `--fail-on`。 |
| `location` | 文件路径 + 1-based 行范围 + 0-based 列范围。`name` 是出问题的符号名（函数名 / 类名）；不适用时为 null。 |
| `message` | 给人看的，带阈值和实际值。 |
| `suggested_refactorings` | 自由形式的标签，对应 Fowler 重构目录里的条目（`"Extract Method"`、`"Replace Conditional with Polymorphism"` 等）。 |
| `actual_value` / `threshold` | 实际值和被越过的阈值。非阈值类 smell（比如 `unsafe_api`）下都是 null。 |
| `risk_score` | 严重度乘以越界程度再乘以结构复杂度因子（取该 finding 所在函数 / 类的相对复杂度）。`cha trend` 子命令在跨 commit 跟踪时用它给 finding 排序。不适用时为 null。 |

schema 文件本身把这些字段标成 required 或 optional，并给 `category`、`severity` 列出可取的枚举值。

## 不走这套 schema 的输出格式

`cha analyze --format json` 是唯一对应这份 schema 的输出。其他格式各有自己的形状：

- **`--format sarif`** 走 [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html)。要校验请用 SARIF 工具，不是 `cha schema`。
- **`--format html`** 是渲染好的 HTML，不存在 schema。
- **`--format llm`** 是 markdown，给 LLM 喂上下文用，不存在 schema。
- **`--format terminal`** 给人看。

## See also

- [`cha init` / `cha schema`](../cli/init.md) —— `cha schema` 子命令本身。
- [JSON 输出](../output/json.md) —— JSON 输出实际长什么样。
- [配置项参考](./config-keys.md) —— `.cha.toml` 的配置 key 表（跟 finding 输出 schema 是两回事）。
- [`Finding` 结构体](https://github.com/W-Mai/Cha/blob/main/cha-core/src/model.rs) —— 权威源。
