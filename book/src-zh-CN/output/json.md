# JSON

机器可读格式。CI 脚本、自定义 dashboard、jq 数据加工都用这个。

## 样例输出

```json
{
  "findings": [
    {
      "smell_name": "lazy_class",
      "category": "dispensables",
      "severity": "hint",
      "actual_value": 0.0,
      "threshold": 1.0,
      "risk_score": 1.5,
      "location": {
        "path": "cha-core/src/cache.rs",
        "start_line": 8,
        "start_col": 7,
        "end_line": 8,
        "end_col": 16,
        "name": "FileEntry"
      },
      "message": "Class `FileEntry` has only 0 method(s) and 8 lines, consider Inline Class",
      "suggested_refactorings": ["Inline Class"]
    }
  ],
  "summary": {
    "files_analyzed": 1,
    "total_lines": 501,
    "tech_debt_minutes": 60,
    "by_severity": { "hint": 15, "warning": 0, "error": 0 }
  }
}
```

字段定义遵循 [JSON Schema 参考](../reference/json-schema.md)（`cha schema` 拿原始 schema 文件）。

## 适用场景

- CI 后处理：用 jq 过滤特定 smell、或者按 severity 聚合
- 自建 dashboard：定时跑 cha 把结果存数据库
- 跟其他工具集成：把 finding 喂进 GitHub Issues / 内部 review 平台

## jq 食谱

```bash
# 只看 warning + error
cha analyze --format json | jq '.findings | map(select(.severity != "hint"))'

# 按 smell 类型计数
cha analyze --format json | jq '.findings | group_by(.smell_name) | map({smell: .[0].smell_name, count: length})'

# 拿出含 baseline 之外新问题的 file 列表
cha analyze --format json --baseline .cha/baseline.json | jq -r '.findings[].location.path' | sort -u
```

## 备注

- 数值字段（`actual_value` / `threshold` / `risk_score`）只在适用时出现——纯 boolean 类型的 smell 不会有这几项
- `summary.tech_debt_minutes` 用 `.cha.toml` 里 `[debt_weights]` 定的分钟数 × 各 severity 数量算出来
