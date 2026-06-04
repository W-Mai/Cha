# HTML

A self-contained HTML report — single file, inline CSS, no external assets. Suitable for archiving, emailing, or hosting on a static file server.

## Sample output

```text
<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>Cha Report</title><style>…</style></head><body>
<header>
  <h1>察 Cha Report</h1>
  <div class="summary">
    <span class="badge error">0 error</span>
    <span class="badge warning">0 warning</span>
    <span class="badge hint">26 hint</span>
    <span class="badge debt">~1h debt</span>
  </div>
</header>
<section>
  <h2>Health Scores</h2>
  <table>
    <tr><th>Grade</th><th>File</th><th>Debt</th><th>Lines</th></tr>
    <tr class="grade-C"><td class="grade">C</td><td><a href="#f-cha-core-src-cache-rs">cha-core/src/cache.rs</a></td><td>~60min</td><td>501</td></tr>
  </table>
</section>
<section>
  <h2>Findings</h2>
  <details id="f-cha-core-src-cache-rs">
    <summary><strong>cha-core/src/cache.rs</strong> <span class="count">(26)</span></summary>
    <div class="source"><table class="code">…<tr class="hl"><td class="ln">8</td><td class="src">struct FileEntry {</td></tr>…</table></div>
    <div class="finding hint"><span class="sev">ℹ️</span> <a href="#f-cha-core-src-cache-rs-L8">[lazy_class] L8-8</a> Class `FileEntry` has only 0 method(s) and 8 lines, consider Inline Class</div>
    …
  </details>
</section>
</body></html>
```

(Captured with `cha analyze --format html --output report.html cha-core/src/cache.rs`. The actual file is one long minified line; whitespace added here for readability.)

## When to use it

- Sharing a snapshot with someone who doesn't have Cha installed locally.
- Archiving a release-time scan (commit the file, or attach to a GitHub release).
- Reviewing a large project's findings in a browser, where collapsible per-file `<details>` sections beat scrolling through terminal output.
- Comparing before/after refactoring — keep two HTML files side by side.

## Notes / Gotchas

- `--output <path>` is **required** for HTML. Without it, `cha analyze --format html` writes nothing useful (HTML markup goes to stdout, but the workflow assumes a file).
- The report is fully self-contained: no JavaScript, no external CSS, no fonts loaded over the network. Drop it on any web host or open via `file://`.
- Each finding's range and ±5 lines of source are inlined into the report, so the HTML grows with file size and finding count. For very large repos, scope with `--top` or per-directory runs.
- Style is dark-mode only by design; the inlined CSS uses GitHub-Dark colours.
- Anchors are stable: `#f-<path-with-slashes-and-dots-replaced-by-dashes>` for files and `#<path-id>-L<line>` for highlighted source rows.
