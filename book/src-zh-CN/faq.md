# 常见问题

### Cha 跟 clippy 啥区别？

clippy 只针对 Rust，主要抓 lint 风格的问题（潜在 bug、写法建议）。Cha 多语言（Rust / TypeScript / Python / Go / C / C++），关心**设计层**的味道——长函数、God class、紧耦合、跨层依赖等等。两个工具互补，不冲突。

### 为啥是 smell 不是 lint？

lint 关心"这段代码可能有 bug"，smell 关心"这段代码设计上有味道"。函数 200 行不会让程序崩，但维护起来费劲——那是味道。Cha 的目标是给"明天接手代码的人"省事，不是给 compiler 找茬。

### 测试 / 自动生成的文件被报了，咋办？

两条路：
- **配置文件全局过滤**：`.cha.toml` 里 `exclude = ["*/tests/fixtures/*", "**/generated/**"]`
- **逐项屏蔽**：在文件 / 函数前面写 `// cha:ignore`（详见 [行内指令](configuration/inline-directives.md)）

测试目录 cha 内置规则有些已经识别了（比如 `__tests__/`、`.test.ts`）但不全。

### 应该 baseline 还是 fix？

- **老代码**：用 [`cha baseline`](cli/baseline.md) 拍快照，老问题屏蔽，新代码不许新增 finding
- **新写的代码**：直接修，别让债累计

混合策略最实用：baseline 用来"冻结历史债"，CI 拦"新增"，每个迭代主动还几条老的。

### `strictness` 怎么用？

整体乘所有数值阈值。`relaxed` = 2.0×、`default` = 1.0×、`strict` = 0.5×，也可以写任意浮点。详见 [严格度与预设](configuration/presets.md)。

不知道项目阈值定多少合适，跑 [`cha calibrate`](cli/calibrate.md)——按你项目实际分布算 P90 / P95 推荐值。

### 怎么禁掉某条 smell？

三种粒度：
- **全局禁**：`.cha.toml` 里 `disabled_smells = ["naming_too_short"]`
- **按语言禁**：`[languages.python] disabled_smells = ["..."]`
- **逐函数 / 类禁**：`// cha:ignore <smell-name>`

### Cha 会改我的代码吗？

只有 `cha fix` 命令会改，**而且当前只支持 `naming_convention` 一种 smell**——把不符合 PascalCase 的类名改对。其他 smell 还得手动修。

`cha analyze` 是只读的。

### 怎么写自己的插件？

写 WASM 插件，详见 [插件开发指南](plugins/development.md)。

### LSP 为啥没补全 / rename？

Cha 不是语言服务器替代品。补全 / rename / 跳转定义这些应该用语言自己的 LSP（rust-analyzer / pyright / gopls）。Cha 跟它们一起跑，专做 finding 诊断、code lens、hover 报告这些 cha 才有的事。

详见 [LSP 概览](lsp/overview.md) "没实现的" 一节。

### 大仓性能咋样？

两级缓存：L1 内存、L2 bincode 磁盘 + mtime 快路径。第二次跑同一个项目（warm）通常亚秒级。新文件 / 改过的文件才重新解析。

### 怎么升级？

按你装 cha 的方式来：

```bash
# Homebrew
brew upgrade W-Mai/cellar/cha-cli

# Shell 安装脚本——直接重跑覆盖
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.sh | sh
```

VS Code 扩展自动跟最新 release。

### 哪种语言覆盖最好？

- **Rust / TypeScript**：最成熟，所有 34 个内置插件都跑
- **Go / Python**：基本完整，少数 OO 类规则不适用
- **C / C++**：tree-sitter 解析支持，但内置 profile 关掉了 OO 类规则（`naming` / `lazy_class` / `data_class`），因为 C 是过程式语言

详见 [严格度与预设](configuration/presets.md) 的"C / C++ profile"一节。
