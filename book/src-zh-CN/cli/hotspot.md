# hotspot

找重构热点——读 git log 拿出"改动频度 × 复杂度"乘积最高的文件。这种文件改得勤又复杂，是技术债集中的地方。

## 用法

```
cha hotspot [参数]
```

## 示例

```bash
# 默认：最近 100 个 commit，前 20 名
cha hotspot

# 看最近 200 个 commit 的前 10 名，输出 JSON
cha hotspot -c 200 -t 10 --format json
```

输出每行带：路径、change frequency、complexity 分、composite score。score 高的优先重构投入产出比最高。

## 参数

| 参数 | 默认 | 说明 |
|------|------|------|
| `-c`, `--count <N>` | `100` | 分析最近 N 个 commit |
| `-t`, `--top <N>` | `20` | 显示前 N 个文件 |
| `--format` | `terminal` | 输出格式：`terminal` / `json` / `llm` / `sarif` / `html` |

## 参考

- [`cha trend`](./trend.md) —— 看 finding 总数随 commit 演变
- [`cha analyze`](./analyze.md) —— 拿到 complexity 数据的源头
