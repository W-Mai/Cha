# Output Formats

`cha analyze` emits findings in five formats. Pick one with `--format <fmt>`; everything else stays the same.

| Format | Best for | CLI flag |
|--------|----------|----------|
| [Terminal](terminal.md) | Local dev, CI logs, quick scan | `--format terminal` (default) |
| [JSON](json.md) | Tooling, scripts, custom dashboards | `--format json` |
| [SARIF](sarif.md) | GitHub Code Scanning, IDE integration | `--format sarif` |
| [HTML](html.md) | Self-contained reports to share / archive | `--format html --output report.html` |
| [LLM](llm.md) | Pasting findings into an AI assistant | `--format llm` |

## Common flags

All formats share the same finding set; these flags shape *which* findings end up in the output:

- `--top N` — show only the N highest-priority findings.
- `--all` — disable per-smell aggregation in terminal output (no effect on other formats).
- `--focus <category>` — restrict to one or more of `bloaters`, `oo_abusers`, `change_preventers`, `dispensables`, `couplers`, `security`.
- `--plugin <name>` — run a single detector.
- `--baseline .cha/baseline.json` — suppress findings present in the baseline.
- `--fail-on hint|warning|error` — exit non-zero when matching findings exist.

`--output` writes to a file instead of stdout. It is required for `--format html` and optional for the rest.
