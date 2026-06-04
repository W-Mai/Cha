# layers

从 import 依赖反推架构层级——把目录归到 tier，看谁在违反"低层不能依赖高层"的规矩。

## 用法

```
cha layers [参数] [路径...]
```

## 示例

```bash
# 跑一遍，终端默认输出（带不稳定度色带的表格）
cha layers --format terminal

# 推断结果存进 .cha.toml 里 [layers] 节
cha layers --save

# DSM 矩阵
cha layers --format dsm

# Mermaid 流程图
cha layers --format mermaid

# 覆盖自动推断的目录深度
cha layers --depth 2
```

跑出来会标出被认定为"违反层级"的边——比如 `domain/` 反过来 import 了 `controller/`。这种边一般就是架构腐化的早期信号。

`--save` 之后，可以跟 `layer_violation` 插件配合（在 `.cha.toml` 启用），让 `cha analyze` 把跨层 import 当 error 拦下来。

## 参数

| 参数 | 默认 | 说明 |
|------|------|------|
| `--format` | `dot` | 输出：`dot` / `json` / `mermaid` / `plantuml` / `dsm` / `terminal` / `html` |
| `--save` | `false` | 把推断的层级写进 `.cha.toml` |
| `--depth <N>` | 自动 | 模块聚合的目录深度 |
| 路径 | `.` | 扫描范围 |

## 参考

- [`cha deps`](./deps.md) —— 画原始 import 图
- [`layer_violation` 插件](../plugins/reference.md#layer_violation) —— 把层级违反作为 finding 报出来
- [配置：layers](../configuration/overview.md) —— 手动写层级
