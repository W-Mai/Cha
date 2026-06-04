# 终端

默认格式，给人眼看的。颜色 + 表情符号区分严重度，同类 findings 自动聚合。

## 样例输出

```text
ℹ [data_class] cha-core/src/cache.rs:8-15 Class `FileEntry` has 4 fields but no behavior methods, consider Move Method
  → suggested: Move Method, Encapsulate Field
ℹ [lazy_class] cha-core/src/cache.rs:8:7-8:16 Class `FileEntry` has only 0 method(s) and 8 lines, consider Inline Class
  → suggested: Inline Class
ℹ [primitive_representation] cha-core/src/cache.rs:144:11-144:15 Function `open` carries domain-named `env_hash: u64` (#2) as raw primitive type(s)
  → suggested: Replace Data Value with Object …
…

15 issue(s) found (0 error, 0 warning, 15 hint).
```

每行：严重度图标 / smell 名 / 路径:行 / 一句话原因。下一行 `→ suggested:` 是推荐的 refactoring。结尾会有总数和按严重度的统计。

## 适用场景

- 本地开发实时跑——`cha analyze` 不带 `--format` 就这个
- 在 PR diff 里看新增问题（搭 `--diff`）
- 命令行检查 `cha analyze --top 10` 抓最严重那批

## 备注

- 终端默认会**聚合**：相同 smell 在同一文件多次出现会折叠。`--all` 关掉聚合，`--top N` 只看最严重 N 条。
- 颜色看 stdin 是否是 TTY 自动启停，pipe 给 `less` 时不会乱码。
