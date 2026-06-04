# 配置

Cha 从项目根目录的 `.cha.toml` 读配置。生成默认模板：

```
cha init
```

## 配置文件位置

`Config::load_for_file` 从被分析文件所在目录往上走，一直到项目根，沿途碰到的每个 `.cha.toml` 都会合并——**离文件越近优先级越高**，根目录是基础。子包能覆盖只关心的几个键。

大部分项目根目录放一份 `.cha.toml` 就够。

## 顶层键

### `plugins`

逐插件配置。所有插件默认开，`enabled = false` 关掉。`[plugins.<名>]` 下的其他键作为选项传给插件。

```toml
[plugins.length]
enabled = true
max_function_lines = 50
max_class_lines = 200

[plugins.coupling]
max_imports = 15
```

数值阈值会被 `strictness` 系数缩放（见下）；字符串和 bool 选项原样透传。完整插件键参考见 [配置项参考](../reference/config-keys.md)。

### `exclude`

要跳过的路径 glob，叠在 `.gitignore` 之上。

```toml
exclude = ["*/tests/fixtures/*", "vendor/*", "**/generated/**"]
```

### `debt_weights`

按 finding 严重度估算技术债的分钟数。analyze summary 用这个算总技术债。默认 `hint = 5`、`warning = 15`、`error = 30`。

```toml
[debt_weights]
hint = 5
warning = 15
error = 30
```

### `strictness`

数值阈值的整体缩放系数：

- `"relaxed"` —— 2.0×（阈值翻倍，更宽松）
- `"default"` —— 1.0×
- `"strict"` —— 0.5×（阈值减半）
- 任意浮点，比如 `0.7`

```toml
strictness = "strict"
# 或者
strictness = 0.7
```

`get_usize` 把缩放后的结果至少夹到 1，所以 `strict` 也不会让阈值变成 0。

### `languages`

按语言覆盖——叠在全局插件配置和内置语言 profile 之上。两个子键：`plugins`（结构跟顶层 `plugins` 一样）和 `disabled_smells`（smell 名列表）。

```toml
[languages.c.plugins.naming]
enabled = false

[languages.c.plugins.length]
max_function_lines = 80

[languages.python]
disabled_smells = ["naming_too_short"]
```

内置 profile（目前 `c` / `cpp`）先应用，你的覆盖优先。详见 [严格度与预设](presets.md)。

### `disabled_smells`

全局禁用 smell 列表。一个插件产出多个 smell 但你只想关其中几条时用这个。

```toml
disabled_smells = ["naming_too_short", "todo_comment"]
```

要更精细到单个函数 / 类的禁用，用 [行内指令](inline-directives.md)。

### `layers`

`cha layers` 用的模块和 tier 定义。不写就让 cha 自动推断。

```toml
[layers.modules]
domain = ["src/domain/**"]
service = ["src/service/**"]
controller = ["src/controller/**"]

[[layers.tiers]]
name = "core"
modules = ["domain"]

[[layers.tiers]]
name = "app"
modules = ["service", "controller"]
```

## 相关页

- [行内指令](inline-directives.md) —— 在源码里用 `// cha:ignore` / `// cha:set`
- [严格度与预设](presets.md) —— strictness 等级和内置语言 profile
- [配置项参考](../reference/config-keys.md) —— 所有键和默认值的完整参考
