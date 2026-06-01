# Cha

<p align="center">
  <img src="static/logo.svg" alt="cha logo" width="160"/>
</p>

<p align="center">
  <strong>察 — 代码健康度分析器</strong>
</p>

<p align="center">
  <a href="README.md">English</a> |
  <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="https://github.com/W-Mai/Cha/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/W-Mai/Cha/ci.yml?style=flat-square" alt="CI" />
  </a>
  <a href="https://github.com/W-Mai/Cha/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/W-Mai/Cha?style=flat-square" alt="License" />
  </a>
  <a href="https://github.com/W-Mai/Cha">
    <img src="https://img.shields.io/github/stars/W-Mai/Cha?style=flat-square" alt="Stars" />
  </a>
  <a href="https://github.com/W-Mai/Cha/releases">
    <img src="https://img.shields.io/github/v/release/W-Mai/Cha?style=flat-square" alt="Release" />
  </a>
</p>

**Cha**（察，「审视、查看」）是一个可插拔的代码坏味道检测工具集。它通过 tree-sitter 在 AST 层解析源码，运行 34 个内置检测器以及用户提供的 WASM 插件，并以终端输出、JSON、LLM 上下文、SARIF 或 HTML 形式呈现结果。

支持语言：Python（`.py`）、TypeScript / TSX（`.ts`、`.tsx`、`.mts`、`.cts`）、Rust（`.rs`）、Go（`.go`）、C（`.c`、`.h`）、C++（`.cpp`、`.cc`、`.cxx`、`.hpp`、`.hxx`）。

## ⚡ 快速开始

```bash
# 分析当前目录（递归，遵循 .gitignore）
cha analyze

# 用 JSON 输出分析指定路径，遇到 error 级别即让 CI 失败
cha analyze src/ --format json --fail-on error

# 仅分析工作区改动过的文件
cha analyze --diff

# 从管道读取 diff 进行分析（适用于 PR review）
gh pr diff | cha analyze --stdin-diff --fail-on warning

# 只跑指定插件
cha analyze --plugin complexity,naming

# 强制全量重新分析（跳过缓存）
cha analyze --no-cache

# 生成基线，之后只汇报新增问题
cha baseline
cha analyze --baseline .cha/baseline.json

# 生成 HTML 报告
cha analyze --format html --output report.html

# 查看解析后的文件结构（函数、类、导入）
cha parse src/

# 生成默认配置 / JSON Schema
cha init
cha schema

# 自动修复简单问题（目前支持：naming_convention 的 PascalCase 重命名）
cha fix src/ --dry-run

# 查看最近 N 个 commit 的问题趋势
cha trend -c 20

# WASM 插件生命周期
cha plugin new my-plugin
cha plugin build
cha plugin install my_plugin.wasm
cha plugin list
cha plugin remove my_plugin

# Shell 补全（fish/bash/zsh/powershell），支持插件名动态补全
cha completions fish > ~/.config/fish/completions/cha.fish

# 查看内置语言预设和严格度等级
cha preset

# 导入 / 类 / 调用关系图（DOT、JSON、Mermaid、PlantUML、DSM、终端、HTML）
cha deps --format dot
cha deps --format mermaid --depth dir
cha deps --type classes --filter Plugin --detail --format plantuml
cha deps --type calls --filter analyze --direction in    # 谁调用了 analyze？
cha deps --type calls --filter analyze --direction out   # analyze 调用了谁？

# 重构热点（基于 git log 的修改频度 × 复杂度）
cha hotspot
cha hotspot -c 200 -t 10 --format json

# 从导入依赖推断架构层级
cha layers
cha layers --format dsm        # 依赖结构矩阵
cha layers --format mermaid
cha layers --depth 2           # 覆盖自动推断的目录深度

# 根据项目统计自动推荐阈值（P90 = warning, P95 = error）
cha calibrate
cha calibrate --apply          # 写入 .cha/calibration.toml（analyze 会自动读取）
```

## ⚡ 性能

Cha 使用两级缓存（L1 内存 + L2 bincode 磁盘），并配合 mtime 快路径，使得对未变更文件的重复分析直接跳过解析。

历史数据，在 NuttX RTOS 的 3,201 个 C 文件上测得，缓存层刚引入时的无缓存 vs. 热缓存对比：

| 命令 | 无缓存 | 热缓存 | 加速 |
|------|-------|-------|------|
| `analyze` | 5.7s | **3.3s** | 26× |
| `layers` | — | **0.8s** | 16× |
| `deps` | — | **0.9s** | 14× |
| `calibrate` | — | **0.6s** | 22× |

## 📦 安装

### Shell（macOS / Linux）

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.sh | sh
```

### PowerShell（Windows）

```powershell
powershell -c "irm https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.ps1 | iex"
```

### Homebrew

```bash
brew install W-Mai/cellar/cha-cli
```

### 从源码构建

```bash
git clone https://github.com/W-Mai/Cha.git
cd Cha
cargo build --release
```

需要 [Rust](https://www.rust-lang.org/tools/install)（edition 2024）。

完整平台和下载方式见 [cha.to01.icu](https://cha.to01.icu)。

## 🔍 内置插件

34 个插件，45 个 smell。少数插件（`length`、`naming`、`error_handling`、`design_pattern`）一项检测会派生多个相关的 smell。下面按 `SmellCategory` 分组列出，CLI 输出、JSON 报告以及 `--focus` 用的也是同一套分类。

所有插件默认启用。在 `[plugins.<name>]` 下设 `enabled = false` 即可关闭。C 语言预设关闭 `naming`、`lazy_class`、`data_class`、`design_pattern`。

下表里的默认值即各插件 `Default for <Analyzer>` 实现里的具体数字；所有阈值都会乘以全局 `strictness` 系数，并可在 `.cha.toml` 里逐插件覆盖，或通过源码中的 `cha:set` 行内指令逐项放宽。

### Bloaters —— 体量过大的代码

| 插件 | smells | 默认阈值 | 严重度 |
|------|--------|---------|-------|
| `length` | `long_method`, `large_class`, `large_file` | `max_function_lines=50`、`max_class_methods=10`、`max_class_lines=200`、`max_file_lines=500`、`complexity_factor_threshold=10.0` | Hint / Warning / Error（按超出幅度递增） |
| `complexity` | `high_complexity` | `warn_threshold=10`、`error_threshold=20` | Warning / Error |
| `cognitive_complexity` | `cognitive_complexity` | `threshold=15`（在基础复杂度上额外按嵌套深度加权） | Warning / Error |
| `long_parameter_list` | `long_parameter_list` | `max_params=5` | Warning |
| `primitive_obsession` | `primitive_obsession` | `min_params=3`、`primitive_ratio=0.8` | Hint |
| `data_clumps` | `data_clumps` | `min_clump_size=3`、`min_occurrences=3` | Hint |
| `naming` | `naming_convention`, `naming_too_short`, `naming_too_long` | `min_name_length=2`、`max_name_length=50` | Hint / Warning |
| `api_surface` | `large_api_surface` | `max_exported_ratio=0.8`、`max_exported_count=20`；C 用 `c_max_exported_ratio=1.01`、`c_max_exported_count=30`、`skip_c_headers=true` | Warning |
| `god_class` | `god_class` | `max_external_refs=5`（ATFD）、`min_wmc=47`、`min_tcc=0.33`（Lanza-Marinescu） | Warning |
| `brain_method` | `brain_method` | `min_lines=65`、`min_complexity=4`、`min_external_refs=7` | Warning |

### Couplers —— 模块之间耦合过紧

| 插件 | smells | 默认阈值 | 严重度 |
|------|--------|---------|-------|
| `coupling` | `high_coupling` | `max_imports=15`；超过 `2 × max_imports` 时升级为 Error | Warning / Error |
| `hub_like_dependency` | `hub_like_dependency` | `max_imports=20` | Warning |
| `feature_envy` | `feature_envy` | `min_refs=3`、`external_ratio=0.7` | Hint |
| `middle_man` | `middle_man` | `min_methods=3`、`delegation_ratio=0.5` | Hint |
| `message_chain` | `message_chain` | `max_depth=3`（`a.b.c.d` 即触发） | Warning |
| `inappropriate_intimacy` | `inappropriate_intimacy` | 检测两个文件之间的双向导入 | Warning |
| `layer_violation` | `layer_violation` | 通过 `layers = "domain:0,service:1,..."` 配置层级；低层不允许导入高层 | Error |
| `async_callback_leak` | `async_callback_leak` | 函数签名泄漏裸 `JoinHandle` / `Future` / `Channel` | Hint |

### OO Abusers —— 面向对象特性使用不当

| 插件 | smells | 默认阈值 | 严重度 |
|------|--------|---------|-------|
| `switch_statement` | `switch_statement` | `max_arms=8`（`switch` / `match` / Python `match` / Go `switch`） | Warning |
| `temporary_field` | `temporary_field` | `min_methods=3`、`max_usage_ratio=0.3` | Hint |
| `refused_bequest` | `refused_bequest` | `min_override_ratio=0.5`、`min_methods=3` | Hint |
| `design_pattern` | `strategy_pattern`, `state_pattern`, `builder_pattern`, `null_object_pattern`, `template_method_pattern`, `observer_pattern` | `strategy_min_arms=4`、`state_min_arms=3`、`builder_min_params=7`（或 `builder_alt_min_params=5` + `builder_alt_min_optional=3`）、`null_object_min_count=3`、`template_min_self_calls=3`、`template_min_methods=4`；类型 / 状态字段关键词列表可配置 | Hint |

### Change Preventers —— 改一处会牵动其他地方

| 插件 | smells | 默认阈值 | 严重度 |
|------|--------|---------|-------|
| `shotgun_surgery` | `shotgun_surgery` | `min_co_changes=5`、`max_commits=100`（读取 `git log`） | Hint |
| `divergent_change` | `divergent_change` | `min_distinct_reasons=4`、`max_commits=50`（读取 `git log`） | Hint |

### Dispensables —— 移除掉也不影响功能的代码

| 插件 | smells | 默认阈值 | 严重度 |
|------|--------|---------|-------|
| `dead_code` | `dead_code` | 未导出 + 无引用的符号；`entry_points` 可配置（默认包含 Rust `main` / `tokio_main`、Python `__init__` / `__main__`、Go `init`、C `_start`） | Hint |
| `duplicate_code` | `duplicate_code` | AST 哈希匹配的重复块 ≥ 10 行 | Warning |
| `comments` | `excessive_comments` | `max_comment_ratio=0.3`、`min_lines=10` | Hint |
| `lazy_class` | `lazy_class` | `max_methods=1`、`max_lines=10` | Hint |
| `data_class` | `data_class` | `min_fields=2` 且方法只是字段访问器 | Hint |
| `speculative_generality` | `speculative_generality` | Interface / trait 实现数 ≤ 1 | Hint |
| `todo_tracker` | `todo_comment` | TODO / FIXME / HACK / XXX 注释；FIXME 升级为 Warning | Hint / Warning |

### Security —— 危险调用与泄漏的密钥

| 插件 | smells | 默认阈值 | 严重度 |
|------|--------|---------|-------|
| `hardcoded_secret` | `hardcoded_secret` | 在 `string_literal` 节点上跑正则；覆盖 API key、token、密码、私钥、JWT | Warning |
| `unsafe_api` | `unsafe_api` | 危险调用：`eval`、`exec`、`system`、`popen`、`sprintf`、`strcpy`、`strcat`、`gets`、`unsafe`、`innerHTML`、`dangerouslySetInnerHTML` | Warning |
| `error_handling` | `empty_catch`, `unwrap_abuse` | `max_unwraps_per_function=3` 限制 `unwrap()` / `expect()`；空 `catch` / `except` 块一律标记 | Warning |

每个插件的 `Default` 实现和 `analyze()` 都在 [`cha-core/src/plugins/`](cha-core/src/plugins)。`cha preset` 列出内置语言预设和严格度等级；`cha analyze --plugin <name>` 单独运行某个检测器。

## ⚙️ 配置

在项目根目录创建 `.cha.toml`：

```toml
# 排除路径（glob）
exclude = ["*/tests/fixtures/*", "vendor/*"]

# 严格度对所有阈值进行整体缩放：
#   relaxed = 2.0×、default = 1.0×、strict = 0.5×，也可填任意 float（例如 0.7）
strictness = "default"

[plugins.length]
enabled = true
max_function_lines = 30
max_class_lines = 200

[plugins.complexity]
warn_threshold = 10
error_threshold = 20

[plugins.coupling]
max_imports = 15

[plugins.layer_violation]
enabled = true
layers = "domain:0,service:1,controller:2"

# 按语言覆写——只写跟全局的差异
[languages.c.plugins.naming]
enabled = false  # C 用 snake_case，跳过 PascalCase 检查

[languages.c.plugins.length]
max_function_lines = 80

# 按严重度估算技术债（分钟），用于 analyze 摘要
[debt_weights]
hint = 5
warning = 15
error = 30
```

### 行内指令

可在源码注释里逐项关闭或放宽规则：

```rust
// cha:ignore                        — 关闭下一项的所有规则
// cha:ignore long_method            — 关闭单条规则
// cha:ignore long_method,complexity — 关闭多条规则
// cha:set long_method=100           — 把 long_method 阈值临时提到 100
// cha:set threshold=200             — 把所有基于阈值的规则一起提到 200
```

支持 `//`、`#`、`/* */` 三种注释风格。

## 🧩 WASM 插件

自定义分析器以 WebAssembly Component Model 模块的形式发布。

```bash
cd examples/wasm-plugin-example
cha plugin build
cha plugin install example.wasm
```

已安装的 `.wasm` 文件放在 `.cha/plugins/`（项目级）或 `~/.cha/plugins/`（全局）。每个插件的选项写在 `.cha.toml` 里：

```toml
[plugins.hardcoded-strings]
SITE_DOMAIN = "example.com"
USER_NAME   = "octocat"
```

### 编写一个插件

`Cargo.toml`：

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
cha-plugin-sdk = { git = "https://github.com/W-Mai/Cha" }
wit-bindgen = "0.55"
```

`src/lib.rs`：

```rust
cha_plugin_sdk::plugin!(MyPlugin);

struct MyPlugin;

impl PluginImpl for MyPlugin {
    fn name() -> String { "my-plugin".into() }
    fn smells() -> Vec<String> { vec!["my_smell".into()] }
    fn analyze(input: AnalysisInput) -> Vec<Finding> { vec![] }
}
```

仓库的 [`examples/`](examples) 下提供四个端到端示例：

- [`wasm-plugin-example`](examples/wasm-plugin-example) —— 检查可疑的函数名
- [`wasm-plugin-hardcoded`](examples/wasm-plugin-hardcoded) —— 检查由配置驱动的硬编码字符串
- [`wasm-plugin-react-hooks`](examples/wasm-plugin-react-hooks) —— React Hooks 规则
- [`wasm-plugin-todo-tracker`](examples/wasm-plugin-todo-tracker) —— TODO/FIXME 跟踪器

📖 **[完整插件开发指南](docs/plugin-development.md)**

## 💡 LSP 集成

```bash
cha lsp
```

已实现的能力（参考 `cha-lsp/src/lib.rs`）：

- **Diagnostics** —— open / change / save 时实时检测
- **Code Actions** —— 推荐重构 + Extract Method
- **CodeLens** —— 在每个函数 / 类上方显示复杂度、行数、参数数
- **Hover** —— Markdown 质量报告卡片
- **Inlay Hints** —— 内联 `cx:N cog:N NL` 标注
- **Document Symbols** —— 大纲视图，对有问题的项标 ⚠
- **Semantic Tokens** —— 给有 finding 的函数 / 类加 warning 修饰
- **Workspace Diagnostics** —— 不打开文件即可全工程扫描
- **Progress** —— 工作区扫描期间发送进度通知

兼容任何支持 LSP 的编辑器（VS Code、Neovim、Helix、Zed、Sublime）。

## 🔌 集成

### Pre-commit

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/W-Mai/Cha
    rev: v1.19.0
    hooks:
      - id: cha-analyze
```

### GitHub Action

```yaml
# .github/workflows/cha.yml
- uses: W-Mai/Cha@v1.19.0
  with:
    fail-on: warning
    upload-sarif: true
```

### VS Code

从 Marketplace 安装 [Cha 扩展](https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha)。首次启动时会自动下载匹配的 `cha` 二进制。

功能：上述全部 LSP 能力 + 自动更新。

## 🛠️ 开发

```bash
# 在本地运行所有 CI 检查
cargo xtask ci

# 单独步骤
cargo xtask build             # release 构建
cargo xtask test              # 单元 + property + fixture 测试
cargo xtask lint              # clippy + fmt
cargo xtask analyze           # 用所有输出格式做自分析
cargo xtask lsp-test          # LSP 冒烟测试
cargo xtask plugin-test       # 插件 SDK + 宏测试
cargo xtask plugin-e2e        # WASM 插件端到端
cargo xtask integration-test  # CLI 集成测试

# 升级 workspace 版本（会改写所有 Cargo.toml + Cargo.lock + vscode-cha/package.json）
cargo xtask bump <major|minor|patch>

# 发版：打 tag → 等 release.yml → 发布到 crates.io
cargo xtask release
```

## 📁 项目结构

```
cha-core/         Plugin trait、注册器、reporter、WASM 运行时、query 辅助
cha-parser/       Tree-sitter 解析（Python、TypeScript、Rust、Go、C、C++）
cha-cli/          CLI 二进制（analyze / parse / deps / layers / hotspot / calibrate / fix / plugin / lsp 等）
cha-lsp/          LSP 服务器
cha-plugin-sdk/   编写 WASM 插件用的 Guest 端 SDK + 宏
xtask/            CI / 发版自动化（cargo xtask）
wit/              WASM 插件的 WIT 接口
examples/         参考 WASM 插件（4 个）
vscode-cha/       VS Code 扩展
docs/             插件开发指南等长文文档
static/           Logo 和资源
```

## 📄 许可证

MIT License.
