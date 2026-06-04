# Vendored assets

| File | Source | Version | License |
|---|---|---|---|
| `mermaid.min.js` | https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js | 11.x (latest at fetch) | MIT |

`mermaid.min.js` is checked in instead of pulled from a CDN at runtime
so the docs site renders without an external network round-trip
(matches the "no CDN" rule applied to other tooling in this repo).

To refresh, re-download from the URL above and overwrite the file.
The companion script `cha-mermaid.js` adapts mdbook's
`<pre><code class="language-mermaid">` output to mermaid's
`<div class="mermaid">` shape and runs `mermaid.run()`.
