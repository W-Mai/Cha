# Summary

[简介](./intro.md)
[安装](./install.md)

# 快速开始

- [命令行](./quick-start/cli.md)
- [pre-commit hook](./quick-start/pre-commit.md)
- [GitHub Actions](./quick-start/github-actions.md)
- [编辑器（LSP）](./quick-start/editor.md)

# 参考

- [配置](./configuration/overview.md)
  - [行内指令](./configuration/inline-directives.md)
  - [严格度与预设](./configuration/presets.md)
  - [配置项参考](./reference/config-keys.md)
- [Smell 列表](./plugins/reference.md)
- [插件开发](./plugins/development.md)
- [命令行](./cli/index.md)
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
  - [完整命令行手册](./reference/cli-manual.md)
- [输出格式](./output/index.md)
  - [终端](./output/terminal.md)
  - [JSON](./output/json.md)
  - [SARIF](./output/sarif.md)
  - [HTML](./output/html.md)
  - [LLM 上下文](./output/llm.md)
  - [JSON Schema](./reference/json-schema.md)
- [LSP 集成](./lsp/overview.md)
  - [VS Code](./lsp/editors/vscode.md)
  - [其他编辑器](./lsp/editors/others.md)

# 烹饪书

- [Recipes](./recipes/index.md)
  - [从 clippy 迁移](./recipes/migrate-clippy.md)
  - [Monorepo CI](./recipes/monorepo-ci.md)
  - [遗留代码豁免](./recipes/suppress-legacy.md)
  - [50 行写一个插件](./recipes/custom-plugin-50loc.md)
  - [给你的项目校准阈值](./recipes/calibrate.md)
  - [Baseline 工作流](./recipes/baseline.md)
- [常见问题](./faq.md)

# 项目

- [贡献](./contributing/index.md)
  - [架构](./contributing/architecture.md)
  - [写一条 smell](./contributing/writing-a-smell.md)
  - [发版](./contributing/releasing.md)
- [学术参考](./references.md)
- [更新日志](./changelog.md)
