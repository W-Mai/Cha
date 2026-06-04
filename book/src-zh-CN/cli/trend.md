# trend

看代码质量随时间的演变——按最近 N 个 commit 各 checkout 一遍跑 analyze，画出 finding 总数 / 严重度分布的曲线。

慢，但一年一两次跑出"我们的技术债到底是涨还是跌"很有用。

## 用法

```
cha trend [参数]
```

## 示例

```bash
# 默认：最近 10 个 commit
cha trend

# 看最近 20 个 commit
cha trend -c 20

# JSON 输出（接 dashboards 用）
cha trend -c 50 --format json
```

每个 commit 都得 checkout + analyze 一遍，跑得不快——大项目 50 commit 可能要几分钟。

## 参数

| 参数 | 默认 | 说明 |
|------|------|------|
| `-c`, `--count <N>` | `10` | 分析最近 N 个 commit |
| `--format` | `terminal` | 输出：`terminal` / `json` / `llm` / `sarif` / `html` |

## 参考

- [`cha hotspot`](./hotspot.md) —— 哪些文件是技术债集中点
- [`cha analyze`](./analyze.md)
