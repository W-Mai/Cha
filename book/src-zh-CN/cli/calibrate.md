# calibrate

按当前项目的实际统计推荐阈值——P90 当 warning 阈值，P95 当 error 阈值。每个项目的"长函数"、"高复杂度"标准其实不一样，calibrate 帮你按自己代码的分布定。

## 用法

```
cha calibrate [--apply] [路径...]
```

## 示例

```bash
# 看建议（不写文件）
cha calibrate

# 把建议写进 .cha/calibration.toml，analyze 会自动读取
cha calibrate --apply
```

输出大概长这样：

```
Metric                Warning(P90)  Error(P95)
long_method                     45          78
high_complexity                  8          14
cognitive_complexity            12          22
```

`--apply` 之后产生的 `.cha/calibration.toml` 会被后续 `cha analyze` 自动叠加在配置之上。想撤回就删掉这个文件。

## 参数

| 参数 | 默认 | 说明 |
|------|------|------|
| `--apply` | `false` | 把建议写进 `.cha/calibration.toml` |
| 路径 | `.` | 统计来源 |

## 参考

- [严格度与预设](../configuration/presets.md) —— `strictness` 是另一种调阈值的方式
- [`cha analyze`](./analyze.md)
- [给你的项目校准阈值](../recipes/calibrate.md)（菜谱）
