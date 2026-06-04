# 其他编辑器

`cha lsp` 是标准 stdio LSP server。任何支持自定义 LSP 服务器的编辑器都能接。下面给最小可用配置——细节查各自编辑器文档。

要让服务器接管的文件类型：`rust` / `typescript` / `typescriptreact` / `python` / `go` / `c` / `cpp`。

## Neovim（nvim-lspconfig）

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.cha then
  configs.cha = {
    default_config = {
      cmd = { 'cha', 'lsp' },
      filetypes = { 'rust', 'typescript', 'typescriptreact', 'python', 'go', 'c', 'cpp' },
      root_dir = lspconfig.util.root_pattern('.cha.toml', '.git'),
      single_file_support = false,
    },
  }
end

lspconfig.cha.setup {}
```

跟你的语言 LSP 同时挂着用——Neovim 会合并多个 server 的诊断。

## Helix（`languages.toml`）

```toml
[language-server.cha]
command = "cha"
args = ["lsp"]

[[language]]
name = "rust"
language-servers = ["rust-analyzer", "cha"]

[[language]]
name = "typescript"
language-servers = ["typescript-language-server", "cha"]

# 同样模式扩到 python / go / c / cpp；tsx 文件 Helix 用 "tsx" 这个 name
```

放 `~/.config/helix/languages.toml`。

## Zed

Zed 用扩展系统管 LSP，最简单的是项目级配置 `.zed/settings.json`：

```json
{
  "lsp": {
    "cha": {
      "binary": { "path": "cha", "arguments": ["lsp"] }
    }
  },
  "languages": {
    "Rust":       { "language_servers": ["rust-analyzer", "cha"] },
    "TypeScript": { "language_servers": ["typescript-language-server", "cha"] },
    "Python":     { "language_servers": ["pyright", "cha"] },
    "Go":         { "language_servers": ["gopls", "cha"] },
    "C":          { "language_servers": ["clangd", "cha"] },
    "C++":        { "language_servers": ["clangd", "cha"] }
  }
}
```

完整 schema 见 [Zed 文档](https://zed.dev/docs/configuring-languages)。

## Sublime Text（LSP 插件）

先装 [LSP 插件](https://packagecontrol.io/packages/LSP)，然后改 `LSP.sublime-settings`：

```json
{
  "clients": {
    "cha": {
      "enabled": true,
      "command": ["cha", "lsp"],
      "selector": "source.rust | source.ts | source.tsx | source.python | source.go | source.c | source.c++"
    }
  }
}
```

selector 语法 + per-window 覆盖见 [LSP 插件文档](https://lsp.sublimetext.io/client_configuration/)。

## 验证连上了

打开支持的文件，故意改超阈值（比如 60+ 行函数触发 `long_method` 默认 50 阈值），保存——应该出诊断。

如果**编辑器里没出但 `cha analyze` CLI 能报**，问题在 LSP 客户端配置——server 跟 cha 都好的，是编辑器没把 server 的诊断接到 UI 上。
