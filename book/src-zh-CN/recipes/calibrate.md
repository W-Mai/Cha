# 给你的项目校准阈值

cha 的默认值（`max_function_lines=50`、`complexity warn=10`）是从 Fowler 的《重构》和 SonarSource 这类静态分析工具里取的经验值。对绿地项目大致合适，对其他几乎所有项目都不太对。

`cha calibrate` 取你这个项目的实际分布作为参考，把第 90 百分位（P90）作为 warning 阈值，第 95 百分位（P95）作为 error 阈值。意思是：比项目里 95% 的代码还复杂的，会让 CI 失败；比 90% 还复杂的，会出 warning；其余安静通过。

## 什么时候跑

- 在已有项目上首次接入 cha。
- 项目长了一两个季度之后（分布会漂）。
- 团队对某条 finding 是否"真的算个事"分歧时——让数据说话。

## 流程

```bash
cha calibrate
```

输出大概长这样：

```
Analyzed 1284 functions across 73 files.

Metric                    Warning(P90) Error(P95)
────────────────────────────────────────────────
long_method                       42         71
high_complexity                    8         13
cognitive_complexity              11         19
```

读法："90% 的函数 ≤ 42 行，95% 的函数 ≤ 71 行"，建议 warn 在 42、error 在 71。

如果数感觉对，存下来：

```bash
cha calibrate --apply
```

这会写出 `.cha/calibration.toml`，里面包含选定的阈值，**以及**每个指标的 P50 / P75 / P90 / P95 完整分布。下次 `cha analyze` 会自动读取。

## 优先级

cha 真正用的阈值，从强到弱：

1. [行内指令](../configuration/inline-directives.md)（`// cha:set max_function_lines=200`）。
2. [`.cha.toml`](../configuration/overview.md) 里的 `[plugins.<name>]`。
3. `.cha/calibration.toml`（`cha calibrate --apply` 写的）。
4. 内置默认值。

如果某个阈值在 `.cha.toml` 里写死了，calibrate 那个数就被忽略——这是有意的，团队明确达成的约定要赢过自动采样。

## 怎么读那份分布文件

`.cha/calibration.toml` 长这样：

```toml
[long_method]
warning = 42
error = 71
p50 = 18
p75 = 31
p90 = 42
p95 = 71

[high_complexity]
warning = 8
error = 13
p50 = 3
p75 = 5
p90 = 8
p95 = 13

[cognitive_complexity]
warning = 11
error = 19
p50 = 4
p75 = 7
p90 = 11
p95 = 19
```

百分位放在文件里就是给你手调用的。如果 P90 = 42、P95 = 71，中间的差值意味着有一个长尾——少数几个特别长的函数。把 `error` 调到 60 能逮住这个尾巴；把 `warning` 调到 50 让普通函数透气。

## strictness 整体倍数

`.cha.toml` 的 `strictness` 给所有阈值（不管是不是 calibrate 出来的）乘一个倍数：

```toml
strictness = "strict"   # 0.5×
strictness = "default"  # 1.0×
strictness = "relaxed"  # 2.0×
strictness = 0.7        # 自定义
```

先 calibrate 一遍，再用 strictness 整体收紧或放宽。

## 局限

`calibrate` 只采样函数级指标。类级（`max_class_lines`、`max_class_methods`）和文件级（`max_file_lines`）阈值不在采样范围内——手填，或者继续用默认。

## See also

- [`cha calibrate`](../cli/calibrate.md)
- [严格度与预设](../configuration/presets.md)
- [配置概览](../configuration/overview.md)
