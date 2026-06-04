# SARIF

[SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) —— 静态分析结果的标准化交换格式。最大用途：上传 GitHub Code Scanning，让 finding 直接出现在 PR 的 "Files changed" 注释和 Security 标签里。

## 样例输出

```json
{
  "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
  "runs": [
    {
      "tool": { "driver": { "name": "cha", "version": "1.19.0" } },
      "properties": {
        "health_scores": [
          { "path": "cha-core/src/cache.rs", "grade": "C", "lines": 501, "debt_minutes": 60 }
        ]
      },
      "results": [
        {
          "level": "note",
          "ruleId": "lazy_class",
          "message": { "text": "Class `FileEntry` has only 0 method(s) and 8 lines, consider Inline Class" },
          "locations": [
            { "physicalLocation": {
              "artifactLocation": { "uri": "cha-core/src/cache.rs" },
              "region": { "startLine": 8, "startColumn": 8, "endLine": 8, "endColumn": 17 }
            }}
          ]
        }
      ]
    }
  ]
}
```

`level` 映射：`hint → note` / `warning → warning` / `error → error`。

## 适用场景

- **GitHub Code Scanning**：CI 跑 `cha analyze --format sarif --output cha.sarif`，再用 [`github/codeql-action/upload-sarif`](https://github.com/github/codeql-action) 上传。finding 自动渲染成 PR 评论、Security 标签里的 alert。
- **GitLab、Azure DevOps** 等也都吃 SARIF
- **企业代码审查平台**（SonarQube、Codacy 等）大多支持 SARIF 导入

## CI 集成例子

```yaml
- run: cha analyze --format sarif --output cha.sarif --fail-on warning
- uses: github/codeql-action/upload-sarif@v3
  if: always()
  with:
    sarif_file: cha.sarif
```

`if: always()` 让上传步骤即使前一步 `--fail-on` 触发了也跑——不然 finding 报告反而上传不了。

## 备注

- `tool.driver.name = "cha"`，所以 GitHub UI 里 finding 的来源会显示成 "cha"
- `properties.health_scores` 是 Cha 的扩展字段（每文件一个 grade A-F + 估算技术债分钟）。SARIF 标准的消费者会忽略这块，但 cha 自己的 dashboard 用得上
- SARIF 是 JSON 的超集，比 `--format json` 多了一层 schema 包装。要纯 finding 数据用 `--format json`
