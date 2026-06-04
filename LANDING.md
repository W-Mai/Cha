<!--
  This file is the homepage source for cha.to01.icu (oranda picks it up
  via project.readme_path in oranda.json). README.md stays focused on
  GitHub readers; this one is the web-landing version: shorter, with
  hero + feature grid + clear CTAs to docs and quick-start.

  Markdown is allowed to contain raw HTML, which is how we get the hero
  block and feature cards without forking oranda's templates. CSS lives
  in oranda.json's styles.additional_css.
-->

<div class="cha-hero">
  <img class="cha-hero-logo" src="/static/logo.svg" alt="Cha logo" onerror="this.style.display='none'">
  <h1 class="cha-hero-title">Cha</h1>
  <p class="cha-hero-sub">察 — Code Health Analyzer</p>
  <p class="cha-hero-tagline">
    Pluggable code-smell detection for real codebases.<br>
    34 built-in detectors, WASM plugins, LSP, six languages, terminal · JSON · SARIF · HTML · LLM-friendly output.
  </p>
  <div class="cha-hero-ctas">
    <a class="cha-btn cha-btn-primary" href="/book/quick-start/cli.html">Quick Start →</a>
    <a class="cha-btn cha-btn-secondary" href="/book/">Read the Docs</a>
    <a class="cha-btn cha-btn-secondary" href="/book/zh-CN/">中文文档</a>
  </div>
  <p class="cha-hero-langs">
    Python · TypeScript / TSX · Rust · Go · C · C++
  </p>
</div>

<div class="cha-features">

<div class="cha-card">
<h3>🔍 34 detectors out of the box</h3>
<p>Bloaters, couplers, OO abusers, change preventers, dispensables, security. Configurable per-plugin, per-language, per-strictness.</p>
<a href="/book/plugins/reference.html">See every smell →</a>
</div>

<div class="cha-card">
<h3>🧩 WASM plugin SDK</h3>
<p>Write a project-specific detector in 50 lines of Rust, compile to <code>wasm32-wasip2</code>, drop the <code>.wasm</code> in <code>.cha/plugins/</code>. No core fork required.</p>
<a href="/book/recipes/custom-plugin-50loc.html">50-line walkthrough →</a>
</div>

<div class="cha-card">
<h3>💡 First-class LSP</h3>
<p>Diagnostics, code actions, code lenses, hover cards, inlay hints, semantic tokens, workspace scan progress. Works with any LSP-aware editor.</p>
<a href="/book/lsp/overview.html">LSP integration →</a>
</div>

<div class="cha-card">
<h3>📊 Git-aware analysis</h3>
<p><code>hotspot</code>, <code>trend</code>, <code>shotgun_surgery</code>, <code>divergent_change</code>, <code>layers</code> — Cha reads your <code>git log</code> to find code that hurts <em>over time</em>, not just on a single snapshot.</p>
<a href="/book/cli/hotspot.html">Refactoring hotspots →</a>
</div>

<div class="cha-card">
<h3>📦 Five output formats</h3>
<p>Terminal for humans, JSON for tools, SARIF for IDEs, HTML for reports, and an LLM-context format for piping into ChatGPT / Claude.</p>
<a href="/book/output/index.html">Output formats →</a>
</div>

<div class="cha-card">
<h3>⚡ Two-level cache</h3>
<p>L1 in-memory + L2 bincode on disk, with an mtime fast-path. 26× speedup on warm runs over 3,201 C files (NuttX RTOS).</p>
<a href="/book/">Performance details →</a>
</div>

</div>

## Get started in 30 seconds

```bash
# Install (macOS / Linux)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.sh | sh

# Analyze the current directory — recursive, .gitignore aware
cha analyze

# CI: only fail on findings introduced after a baseline
cha baseline                                    # snapshot once
cha analyze --baseline .cha/baseline.json --fail-on warning
```

Other install options ([Homebrew, PowerShell, from source](/artifacts/)) and [pre-commit / GitHub Actions integration](/book/quick-start/github-actions.html) are documented in the docs.

## What can `cha` catch?

| Category | Detects | Example smells |
|---|---|---|
| **Bloaters** | Code that has grown too large | `long_method`, `god_class`, `complexity`, `cognitive_complexity` |
| **Couplers** | Modules tied too tightly | `coupling`, `feature_envy`, `hub_like_dependency`, `layer_violation` |
| **OO Abusers** | Object-oriented constructs misused | `switch_statement`, `refused_bequest`, `design_pattern` (Strategy / State / Builder / etc.) |
| **Change Preventers** | One change forces N edits elsewhere | `shotgun_surgery`, `divergent_change` |
| **Dispensables** | Code that can be removed | `dead_code`, `duplicate_code`, `lazy_class`, `data_class` |
| **Security** | Risky calls, leaked secrets | `hardcoded_secret`, `unsafe_api`, `empty_catch`, `unwrap_abuse` |

[Full list with thresholds and triggering examples →](/book/plugins/reference.html)

## Beyond linting

Cha is more than a linter. It also produces:

- **Dependency graphs** (`cha deps`) — DOT, Mermaid, PlantUML, JSON, terminal, HTML.
- **Refactoring hotspots** (`cha hotspot`) — change frequency × complexity, scored from `git log`.
- **Architecture layer inference** (`cha layers`) — auto-detect tiers from import graphs, render as DSM matrix or Mermaid.
- **Threshold calibration** (`cha calibrate`) — sample your project's P90 / P95, propose data-driven thresholds.
- **Auto-fix** (`cha fix --dry-run`) — rename PascalCase, more rules over time.

## Editor integration

[**VS Code Marketplace** — install in one click](https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha)

Or any editor that speaks LSP — Neovim, Helix, Zed, Sublime — see the [LSP integration guide](/book/lsp/overview.html).

## License & links

Cha is MIT-licensed and developed in the open at [github.com/W-Mai/Cha](https://github.com/W-Mai/Cha). The full README — including every plugin's thresholds, configuration shape, and a longer feature tour — lives there. The [docs](/book/) (this site) cover the same ground in walkthrough form.

Star ⭐ on GitHub if Cha helps your codebase.
