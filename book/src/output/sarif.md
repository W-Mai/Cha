# SARIF

[SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) — the format GitHub Code Scanning, GitLab, and most static-analysis aggregators consume. Use this whenever you want findings to appear as inline annotations on pull requests.

## Sample output

```text
{
  "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "cha",
          "version": "1.19.0",
          "rules": [
            { "id": "lazy_class", "shortDescription": { "text": "lazy_class" } },
            { "id": "data_class", "shortDescription": { "text": "data_class" } }
          ]
        }
      },
      "results": [
        {
          "ruleId": "lazy_class",
          "level": "note",
          "message": { "text": "Class `FileEntry` has only 0 method(s) and 8 lines, consider Inline Class" },
          "locations": [{
            "physicalLocation": {
              "artifactLocation": { "uri": "cha-core/src/cache.rs" },
              "region": { "startLine": 8, "startColumn": 8, "endLine": 8, "endColumn": 17 }
            }
          }]
        }
        …
      ],
      "properties": {
        "health_scores": [{ "path": "cha-core/src/cache.rs", "grade": "C", "debt_minutes": 60, "lines": 501 }]
      }
    }
  ]
}
```

(Captured with `cha analyze --format sarif cha-core/src/cache.rs`.)

## When to use it

- **GitHub Code Scanning** — upload via [`github/codeql-action/upload-sarif`](https://github.com/github/codeql-action) and findings render as inline PR annotations and feed into the Security tab.
- GitLab, Azure DevOps, Sonar, and other CI platforms that ingest SARIF natively.
- IDEs and viewers (VS Code's [SARIF Viewer](https://marketplace.visualstudio.com/items?itemName=MS-SarifVSCode.sarif-viewer), JetBrains Qodana) for opening a saved scan.
- Aggregating findings from multiple tools — SARIF runs can be merged into one report.

## Notes / Gotchas

- Severities map: `error` → `error`, `warning` → `warning`, `hint` → `note`.
- SARIF columns are **1-based**; Cha's internal 0-based columns are incremented on the way out. Don't double-shift if you compare against the JSON output.
- Each unique `smell_name` appears once in `tool.driver.rules` and is referenced from each result via `ruleId`.
- Health scores are non-standard SARIF, attached under `runs[0].properties.health_scores`. Generic SARIF consumers ignore unknown properties.
- GitHub Code Scanning has a **5,000 result limit** per upload; combine with `--top` or `--baseline` on noisy projects.
- Minimal CI integration:

  ```yaml
  - run: cha analyze --format sarif --output cha.sarif
  - uses: github/codeql-action/upload-sarif@v3
    with:
      sarif_file: cha.sarif
  ```
