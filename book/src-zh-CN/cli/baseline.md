# baseline

把当前所有 finding 拍个快照。后续 `cha analyze --baseline` 只报快照之后**新增**的 finding，老问题屏蔽。

接手老仓库时最常用——legacy 代码不可能一次清完，但又不想新代码退化。

## 用法

```
cha baseline [-o <文件>] [路径...]
```

## 示例

```bash
# 在当前目录生成 baseline，默认写 .cha/baseline.json
cha baseline

# 指定输出位置
cha baseline -o legacy-baseline.json

# 之后 CI 里只看 baseline 之外的新问题
cha analyze --baseline .cha/baseline.json --fail-on warning
```

典型工作流：项目第一次接 Cha 时跑 `cha baseline` 提交 baseline 文件，CI 用 `--baseline` 跑 analyze。新代码一旦引入新 finding，CI 就拦下来；老代码慢慢治理。

## 参数

| 参数 | 默认 | 说明 |
|------|------|------|
| `-o`, `--output <path>` | `.cha/baseline.json` | baseline 文件路径 |
| 路径 | `.` | 扫描范围 |

## 参考

- [Baseline 工作流](../recipes/baseline.md)（菜谱页）
- [`cha analyze --baseline`](./analyze.md)
