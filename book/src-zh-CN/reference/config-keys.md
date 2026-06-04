# 配置项参考

`.cha.toml` 能识别的全部 key。结构由 [`cha_core::Config`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/config.rs) 定义；多余的 key 会被静默忽略。

## 顶层

| Key | 类型 | 默认 | 含义 |
|---|---|---|---|
| `exclude` | `Vec<String>` | `[]` | 跳过的路径 glob 模式。模式是相对项目根的路径。`**` 匹配任意层。文件遍历器本身就尊重 `.gitignore`，所以 `node_modules/` 这种通常不用写。 |
| `strictness` | `"relaxed"` / `"default"` / `"strict"` / `f64` | `"default"` | 阈值整体倍数。`relaxed = 2.0×`，`default = 1.0×`，`strict = 0.5×`，或者写一个浮点数（`0.7`）。只作用于整数阈值；比例类阈值不被这个倍数缩放。 |
| `disabled_smells` | `Vec<String>` | `[]` | 全局屏蔽的 smell 名。当一个插件出多条 smell（`length` 一个插件出 `long_method` / `large_class` / `large_file`）但你只想屏蔽其中几条时用。 |
| `debt_weights` | table | 见下文 | 每个严重度对应的 tech-debt 分钟数。`cha analyze` 摘要行用。 |
| `plugins` | table of tables | （默认值） | 插件级覆盖。见 [插件级 section](#插件级-section)。 |
| `languages` | table of tables | `{}` | 语言级覆盖。见 [语言级 section](#语言级-section)。 |
| `layers` | table | （空） | `cha layers` 的模块 / tier 定义。见 [Layers section](#layers-section)。 |

### `debt_weights`

```toml
[debt_weights]
hint = 5        # 默认 5
warning = 15    # 默认 15
error = 30      # 默认 30
```

单位是分钟。摘要行的总 debt 显示为 `<n>h <n>m`。

## 插件级 section

每个插件从 `[plugins.<name>]` 读自己的配置。结构：

```toml
[plugins.<name>]
enabled = true             # 默认 true；写 false 关掉
# ... 插件自己的 key
```

插件自己的 key 各不相同，默认值跟代码里 `Default` impl 对齐。完整对照：

| `[plugins.<name>]` | 可配 key | 备注 |
|---|---|---|
| `length` | `max_function_lines` (50)、`max_class_methods` (10)、`max_class_lines` (200)、`max_file_lines` (500)、`complexity_factor_threshold` (10.0) | 越界越多严重度越高。 |
| `complexity` | `warn_threshold` (10)、`error_threshold` (20) | Cyclomatic complexity。 |
| `cognitive_complexity` | `threshold` (15) | 超过 `2 × threshold` 升级为 `Error`。 |
| `long_parameter_list` | `max_params` (5) | |
| `primitive_obsession` | `min_params` (3)、`primitive_ratio` (0.8) | |
| `data_clumps` | `min_clump_size` (3)、`min_occurrences` (3) | |
| `naming` | `min_name_length` (2)、`max_name_length` (50) | |
| `api_surface` | `max_exported_ratio` (0.8)、`max_exported_count` (20)、`c_max_exported_ratio` (1.01)、`c_max_exported_count` (30)、`skip_c_headers` (true) | C 单独放宽：header 文件本来就是用来对外暴露 API 的。 |
| `god_class` | `max_external_refs` (5)、`min_wmc` (47)、`min_tcc` (0.33) | 三项分别是 ATFD（access to foreign data，对外部数据的访问数）、WMC（weighted method count，加权方法数）、TCC（tight class cohesion，类内紧密内聚度），来自 Lanza & Marinescu 的 *Object-Oriented Metrics in Practice*。 |
| `brain_method` | `min_lines` (65)、`min_complexity` (4)、`min_external_refs` (7) | |
| `coupling` | `max_imports` (15) | 超过 `2 × max_imports` 升级为 `Error`。 |
| `hub_like_dependency` | `max_imports` (20) | |
| `feature_envy` | `min_refs` (3)、`external_ratio` (0.7) | |
| `middle_man` | `min_methods` (3)、`delegation_ratio` (0.5) | |
| `message_chain` | `max_depth` (3) | |
| `inappropriate_intimacy` | （无阈值） | 检测两文件互相 import。 |
| `layer_violation` | `layers = "domain:0,service:1,..."` | 字符串格式：`name:rank,name:rank,...`。低 rank 不能 import 高 rank。 |
| `async_callback_leak` | （无阈值） | 检测公开签名里裸出现的 `JoinHandle` / `Future` / `Channel`。 |
| `switch_statement` | `max_arms` (8) | |
| `temporary_field` | `min_methods` (3)、`max_usage_ratio` (0.3) | |
| `refused_bequest` | `min_override_ratio` (0.5)、`min_methods` (3) | |
| `design_pattern` | `strategy_min_arms` (4)、`state_min_arms` (3)、`builder_min_params` (7)、`builder_alt_min_params` (5)、`builder_alt_min_optional` (3)、`null_object_min_count` (3)、`template_min_self_calls` (3)、`template_min_methods` (4)，以及若干关键词列表 | 一个插件出 6 个 pattern smell，阈值各管各的。 |
| `shotgun_surgery` | `min_co_changes` (5)、`max_commits` (100) | 读 `git log`。 |
| `divergent_change` | `min_distinct_reasons` (4)、`max_commits` (50) | 读 `git log`。 |
| `dead_code` | `entry_points` (各语言默认) | 列在这里的函数永远不会被判定为死代码。 |
| `duplicate_code` | `min_lines` (10) | AST hash 比对。 |
| `comments` | `max_comment_ratio` (0.3)、`min_lines` (10) | |
| `lazy_class` | `max_methods` (1)、`max_lines` (10) | |
| `data_class` | `min_fields` (2) | |
| `speculative_generality` | （无阈值） | interface / trait 实现 ≤ 1 个。 |
| `todo_tracker` | （无阈值） | TODO / FIXME / HACK / XXX。HACK 和 XXX 升级为 Warning。 |
| `hardcoded_secret` | （内置正则集） | API key、token、密码、JWT。 |
| `unsafe_api` | （内置调用集） | `eval`、`exec`、`system`、`strcpy`、`gets`、`unsafe`、`innerHTML`、`dangerouslySetInnerHTML` 等。 |
| `error_handling` | `max_unwraps_per_function` (3) | 空 `catch` 永远报。 |

权威默认值在每个插件的 `Default for <Analyzer>` impl 里，[`cha-core/src/plugins/`](https://github.com/W-Mai/Cha/tree/main/cha-core/src/plugins) 下找。

## 语言级 section

```toml
[languages.<lang>]
disabled_smells = []
[languages.<lang>.plugins.<name>]
# ... 跟 [plugins.<name>] 同样的 key
```

`<lang>` 是 cha 给文件打的语言 ID：`python`、`typescript`、`tsx`、`rust`、`go`、`c`、`cpp`。语言级配置会**覆盖**全局值，不会跟全局值合并。C 的[内置预设](../configuration/presets.md)默认关闭 `naming`、`lazy_class`、`data_class`、`design_pattern`，并把 `length.max_function_lines` 提到 80。

例：

```toml
[languages.c.plugins.naming]
enabled = false

[languages.c.plugins.length]
max_function_lines = 80

[languages.python.plugins.long_parameter_list]
max_params = 8                          # Python 的 **kwargs 容忍多一点
```

## Layers section

给 `cha layers` 和 `layer_violation` smell 用的：

```toml
[layers]
modules = { domain = ["src/domain/**"], service = ["src/service/**"], controller = ["src/handlers/**"] }

[[layers.tiers]]
name = "data"
modules = ["domain"]

[[layers.tiers]]
name = "logic"
modules = ["service"]

[[layers.tiers]]
name = "api"
modules = ["controller"]
```

文件里 tier 的顺序 = 从底到顶。低 tier 不能 import 高 tier。`layer_violation` 单插件本身有更简单的 inline 写法（`[plugins.layer_violation]` 下的 `layers = "..."` 字符串），见上表。

## 行内指令

直接在源码里覆盖配置。完整语法见 [行内指令](../configuration/inline-directives.md)：

```rust
// cha:ignore                        — 屏蔽下一个 item 上的所有规则
// cha:ignore long_method            — 屏蔽一条
// cha:ignore long_method,complexity — 屏蔽多条
// cha:set long_method=100           — 单独把 long_method 阈值提到 100
// cha:set threshold=200             — 把所有阈值类规则的阈值提到 200
```

支持 `//`、`#`、`/* */` 三种注释。

## See also

- [配置概览](../configuration/overview.md)
- [严格度与预设](../configuration/presets.md)
- [行内指令](../configuration/inline-directives.md)
- [JSON Schema](./json-schema.md) —— 那是**输出 schema**，不是 config schema。
- [`cha-core/src/config.rs`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/config.rs) —— 权威源。
