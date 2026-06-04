# deps

画依赖图——文件之间的 import、类继承、函数调用三选一。出图格式有 DOT / Mermaid / PlantUML / DSM / 终端 ASCII / HTML / JSON。

## 用法

```
cha deps [参数] [路径...]
```

## 示例

```bash
# 默认：导入依赖图，DOT 格式
cha deps --format dot

# Mermaid 流程图，按目录粒度聚合
cha deps --format mermaid --depth dir

# 类继承图
cha deps --type classes --format dot

# 只看名字含 Plugin 的类，输出 PlantUML
cha deps --type classes --filter Plugin --detail --format plantuml

# 调用图：谁调用了 analyze？
cha deps --type calls --filter analyze --direction in

# analyze 调用了谁？
cha deps --type calls --filter analyze --direction out
```

## 参数

| 参数 | 默认 | 说明 |
|------|------|------|
| `--type` | `imports` | 图的类型：`imports` / `classes` / `calls` |
| `--format` | `dot` | 输出：`dot` / `json` / `mermaid` / `plantuml` / `dsm` / `terminal` / `html` |
| `--depth` | 自动 | 聚合粒度：`file` / `dir` / 数字（自定义层级深度） |
| `--filter <名>` | — | 只看名字含 `<名>` 的节点 |
| `--exact` | `false` | `--filter` 改为完全匹配 |
| `--detail` | `false` | 类图：连同字段、方法签名一起出 |
| `--direction` | `both` | `--type calls` 专用：`in`（被调）/ `out`（调用别人）/ `both` |
| 路径 | `.` | 扫描范围 |

`dsm` 是依赖结构矩阵——大项目看模块间循环很有用。

## 参考

- [`cha layers`](./layers.md) —— 推断架构层级
- [`cha hotspot`](./hotspot.md) —— 改动 × 复杂度热点
