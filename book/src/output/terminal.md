# Terminal

Default format. Designed for humans reading a shell, and for CI logs where someone scrolls back to find what broke.

## Sample output

```text
ℹ [lazy_class] cha-core/src/cache.rs:8:7-8:16 Class `FileEntry` has only 0 method(s) and 8 lines, consider Inline Class
  → suggested: Inline Class
ℹ [lazy_class] cha-core/src/cache.rs:19:7-19:20 Class `FindingsEntry` has only 0 method(s) and 4 lines, consider Inline Class
  → suggested: Inline Class
ℹ [lazy_class] cha-core/src/cache.rs:26:7-26:16 Class `CacheMeta` has only 0 method(s) and 4 lines, consider Inline Class
  → suggested: Inline Class
ℹ [lazy_class] cha-core/src/cache.rs:298:9-298:19 Class `FileStatus` has only 0 method(s) and 6 lines, consider Inline Class
  → suggested: Inline Class
ℹ [data_class] cha-core/src/cache.rs:8-15 Class `FileEntry` has 4 fields but no behavior methods, consider Move Method
  → suggested: Move Method, Encapsulate Field

26 issue(s) found (0 error, 0 warning, 26 hint). (showing top 5)

Health scores:
  C cha-core/src/cache.rs (~60min debt)

Tech debt: ~1h | A:0 B:0 C:1 D:0 F:0
```

(Captured with `cha analyze --format terminal --top 5 cha-core/src/cache.rs`.)

Each finding is one line: `<icon> [smell] <path>:<line>[:<col>][-<endline>[:<endcol>]] <message>`. Icons are `✗` for error, `⚠` for warning, `ℹ` for hint. The optional `→ suggested: …` line below names refactorings the detector recommends.

## When to use it

- Local development — running `cha analyze` directly in a terminal.
- CI log output where humans (not parsers) read the result.
- Pre-commit hooks: short, scannable, easy to spot in a wall of git output.
- Quick triage with `--top 10` to see the highest-priority issues first.

## Notes / Gotchas

- When the same smell fires more than 5 times in a single run, the default view aggregates: it prints the smell name once with an occurrence count and shows the top 3 most severe locations followed by `… and N more (use --all to show all)`. Pass `--all` to expand every group.
- `--top N` truncates after sorting by risk score and adds `(showing top N)` to the summary.
- The summary line is always printed; an empty run produces `No issues found.`.
- Health scores and tech-debt totals appear after the findings, derived from the same data.
