# 严格度与预设

调默认值有两条路：全局**严格度系数**（一刀切乘所有阈值）和**按语言预设**（内置 profile + 你的覆盖）。

## 严格度

`.cha.toml` 里写 `strictness`（或者 CLI 上 `--strictness`），整体乘所有数值阈值：

| 值 | 系数 | 效果 |
|----|------|------|
| `"relaxed"` | 2.0× | 阈值翻倍——更宽松 |
| `"default"` | 1.0× | 用插件出厂默认 |
| `"strict"` | 0.5× | 阈值减半——更严 |
| 任意浮点 `0.7` | 字面量 | 自定义 |

```toml
strictness = "strict"
# 或者
strictness = 0.7
```

`relaxed` / `default` / `strict` 是命名档；其他值按浮点解析。结果至少夹到 1，`strict` 也不会让阈值变 0。

只有插件的整数选项（函数长度上限、复杂度阈值、参数数等）会被缩放。比例类的（`external_ratio`、`primitive_ratio` 这种 0-1 之间的）按字面量读，不缩放。

## 内置语言 profile

Cha 给这几种语言带了内置 profile：`c` / `cpp` / `python` / `typescript` / `rust` / `go`。看一下：

```bash
cha preset list           # 哪些语言有 profile，各禁用了多少 smell
cha preset show c         # C 的完整解析配置：插件 / smell / 严格度
cha preset show rust
```

目前**只有 `c` / `cpp` profile 真的改默认值**——其他语言的 profile 列表里有，但没有覆盖。

### C / C++ profile

C 是过程式语言，OO 类的检测器默认关掉：

- **插件级禁用**（完全不出 finding）：`naming` / `lazy_class` / `data_class`
- **smell 级禁用**（插件还跑，但这几条 smell 过滤掉）：`builder_pattern` / `null_object_pattern` / `strategy_pattern` / `data_clumps`

profile 也把大小和耦合阈值调高了——C 项目函数本来就长，include 也多：

| 插件 | 选项 | C / C++ 值 |
|------|------|----------|
| `length` | `max_function_lines` | 100 |
| `length` | `max_file_lines` | 2000 |
| `length` | `max_class_lines` | 400 |
| `complexity` | `warn_threshold` | 15 |
| `complexity` | `error_threshold` | 30 |
| `cognitive_complexity` | `threshold` | 25 |
| `coupling` | `max_imports` | 25 |
| `long_parameter_list` | `max_params` | 7 |

## 自己覆盖

`[languages.<lang>]` 下面写啥都覆盖该语言的内置 profile。结构跟全局一样：

```toml
# C 上重新打开 naming，但放宽最短名长度
[languages.c.plugins.naming]
enabled = true
min_name_length = 3

# Python 不禁用任何插件，但单独丢掉一条 smell
[languages.python]
disabled_smells = ["naming_too_short"]

# Rust 单独把函数行数上限调严，不动全局 strictness
[languages.rust.plugins.length]
max_function_lines = 40
```

你写的键优先，内置 profile 在你之下——想重新打开 profile 关掉的插件，写 `enabled = true` 就行，不用别的仪式。

要看你覆盖之后某语言到底什么配置：

```bash
cha preset show <语言>
```

输出包含解析后的严格度系数、最终启用的所有插件、profile 关掉的插件、以及你额外加的 `disabled_smells`。

## 相关

- [配置概览](overview.md) —— 顶层键
- [行内指令](inline-directives.md) —— 不动配置文件的逐项覆盖
- [`cha preset`](../cli/preset.md) —— 命令文档
