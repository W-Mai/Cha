# Summary

[Introduction](./intro.md)
[Install](./install.md)

# Quick Start

- [CLI](./quick-start/cli.md)
- [Pre-commit hook](./quick-start/pre-commit.md)
- [GitHub Actions](./quick-start/github-actions.md)
- [Editor (LSP)](./quick-start/editor.md)

# Reference

- [Configuration](./configuration/overview.md)
  - [Inline directives](./configuration/inline-directives.md)
  - [Strictness and presets](./configuration/presets.md)
  - [Configuration keys](./reference/config-keys.md)
- [Smells](./plugins/reference.md)
- [Plugin development](./plugins/development.md)
- [CLI](./cli/index.md)
  - [analyze](./cli/analyze.md)
  - [parse](./cli/parse.md)
  - [baseline](./cli/baseline.md)
  - [fix](./cli/fix.md)
  - [deps](./cli/deps.md)
  - [layers](./cli/layers.md)
  - [hotspot](./cli/hotspot.md)
  - [trend](./cli/trend.md)
  - [calibrate](./cli/calibrate.md)
  - [preset](./cli/preset.md)
  - [plugin](./cli/plugin.md)
  - [completions](./cli/completions.md)
  - [init / schema](./cli/init.md)
  - [Full CLI manual](./reference/cli-manual.md)
- [Output formats](./output/index.md)
  - [Terminal](./output/terminal.md)
  - [JSON](./output/json.md)
  - [SARIF](./output/sarif.md)
  - [HTML](./output/html.md)
  - [LLM context](./output/llm.md)
  - [JSON Schema](./reference/json-schema.md)
- [LSP integration](./lsp/overview.md)
  - [VS Code](./lsp/editors/vscode.md)
  - [Other editors](./lsp/editors/others.md)

# Cookbook

- [Recipes](./recipes/index.md)
  - [Migrate from clippy](./recipes/migrate-clippy.md)
  - [CI on a monorepo](./recipes/monorepo-ci.md)
  - [Suppress in legacy code](./recipes/suppress-legacy.md)
  - [Custom plugin in 50 lines](./recipes/custom-plugin-50loc.md)
  - [Calibrate to your codebase](./recipes/calibrate.md)
  - [Baseline workflow](./recipes/baseline.md)
- [FAQ](./faq.md)

# Project

- [Contributing](./contributing/index.md)
  - [Architecture](./contributing/architecture.md)
  - [Writing a smell](./contributing/writing-a-smell.md)
  - [Releasing](./contributing/releasing.md)
- [Academic references](./references.md)
- [Changelog](./changelog.md)
