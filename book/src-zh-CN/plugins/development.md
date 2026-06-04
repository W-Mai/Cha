# 插件开发指南

Cha 是代码坏味道（code smell）检测工具。除了 34 个内置检测器，还可以装第三方插件——一个 WASM 模块，host 把每个文件解析后丢给你的 `analyze()`，你返回若干 `Finding`（检测到的坏味道），host 汇合所有 finding 输出给用户。

这一页讲怎么从零写、编译、装、测一个这样的插件。

## 前置

- Rust 工具链 + `wasm32-wasip1` target：

  ```bash
  rustup target add wasm32-wasip1
  ```

- `cha` CLI 装好且在 `$PATH` 里

## Quick Start

```bash
mkdir my-plugin && cd my-plugin
cha plugin new my-plugin
cha plugin build           # 产出 my_plugin.wasm
cha plugin install my_plugin.wasm
cha analyze src/
```

包名是 `my-plugin`，编译产物是 `my_plugin.wasm`——Cargo 把 `-` 转成 `_`。后面 `cha analyze` 自动加载装好的所有插件。

## 脚手架

`cha plugin new <名字>` 生成：

```
my-plugin/
  Cargo.toml   # cdylib + cha-plugin-sdk + wit-bindgen 依赖
  src/
    lib.rs     # plugin! 宏 + 一个 PluginImpl 最简实现
```

当前目录空就在原地生成；不空就建一个 `<名字>/` 子目录。

## 插件结构

```rust
cha_plugin_sdk::plugin!(MyPlugin);

struct MyPlugin;

impl PluginImpl for MyPlugin {
    fn name() -> String { "my-plugin".into() }
    fn smells() -> Vec<String> { vec!["my_smell".into()] }
    fn analyze(input: AnalysisInput) -> Vec<Finding> { vec![] }
}
```

`plugin!` 宏会替你接好 host 跟插件之间的通信、把下面要用到的所有类型 import 进作用域。你只用关心 `PluginImpl` trait 怎么实现。

### 类型清单

`plugin!(MyPlugin)` 之后，下面这些类型自动在作用域里。`PluginImpl` 是你要实现的 trait。

| 类型 | 说明 |
|------|------|
| `AnalysisInput` | `analyze()` 拿到的完整文件上下文 |
| `Finding` | 一条 finding |
| `FunctionInfo` | 单个函数的信息（名字、行号、复杂度、参数等） |
| `ClassInfo` | 单个类的信息（方法数、字段、是否导出等） |
| `ImportInfo` | import 来源 + 行号 + 是否是模块声明 |
| `CommentInfo` | 注释文本 + 行号 |
| `ArmValue` | switch / match 分支的字面值（`StrLit` / `IntLit` / `CharLit` / `Other`） |
| `FileRole` | `Source` / `Test` / `Doc` / `Config` / `Generated` |
| `Location` | 文件路径 + 行列范围 |
| `Severity` | `Hint` / `Warning` / `Error` |
| `SmellCategory` | `Bloaters` / `Couplers` / `Dispensables` / ... |
| `OptionValue` | 配置值类型：`Str` / `Int` / `Float` / `Boolean` / `ListStr` |
| `tree_query` | AST query 模块（见下） |
| `project_query` | 跨文件查询模块（见下） |

### AnalysisInput 字段

```rust
pub struct AnalysisInput {
    pub path: String,             // 文件路径
    pub content: String,          // 源码原文
    pub language: String,         // "typescript" | "rust" | "python" | "go" | "c" | "cpp"
    pub total_lines: u32,
    pub role: FileRole,           // Source / Test / Doc / Config / Generated
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<ImportInfo>,
    pub comments: Vec<CommentInfo>,
    pub type_aliases: Vec<(String, String)>,
    pub options: Vec<(String, OptionValue)>,  // 来自 .cha.toml
}
```

> **Warning**：WASM 插件跑在沙箱里，**没有文件系统权限**。读源码用 `input.content`，**不要**用 `std::fs::read_to_string(&input.path)`——会静默返回空字符串。

### 文件角色

`role` 字段告诉你正在分析的是什么类型的文件。利用它给不同文件套不同规则：

```rust
fn analyze(input: AnalysisInput) -> Vec<Finding> {
    if input.role == FileRole::Test {
        return vec![];  // 测试文件跳过
    }
    // ...
}
```

### Declaring smells

每个 `Finding` 都带一个 `smell_name`。在 `smells()` 里把全部 smell 名声明出来，host 就能：

- 在 `cha plugin list` 里展示你这个插件能出哪些 smell
- 让用户在 `.cha.toml` 里写 `disabled_smells = ["你的_smell"]` 来禁用某条
- 把禁用名单回传给你的插件，你早点跳过这部分计算

`input.options` 里有个特殊 key `__disabled_smells__` 装着用户禁用的 smell 名单。提前跳过：

```rust
use cha_plugin_sdk::is_smell_disabled;

fn analyze(input: AnalysisInput) -> Vec<Finding> {
    let mut out = Vec::new();
    if !is_smell_disabled!(&input.options, "my_smell") {
        // 没被禁的时候才算
    }
    out
}
```

`is_smell_disabled!` 是个宏（注意感叹号）。它返回 `bool`。

host 也会**事后再过滤**一遍 finding，所以忘调 `is_smell_disabled!` 不会让被禁的 smell 漏到用户输出——只是白算一遍。

### AST Query API（`tree_query`）

插件可以通过 host 回调跑 tree-sitter query，查当前文件的 AST：

```rust
fn analyze(input: AnalysisInput) -> Vec<Finding> {
    // 找文件里所有 unsafe 块
    // 返回 Vec<Vec<QueryMatch>> —— 外层每个 match 一项，内层每个 capture 一项
    let matches: Vec<Vec<QueryMatch>> = tree_query::run_query("(unsafe_block) @blk");
    for m in &matches {
        for capture in m {
            // capture.node_kind / capture.text / capture.start_line ...
        }
    }

    // 一次跑多个 query（减少 host 边界穿越开销）
    // 返回 Vec<Vec<Vec<QueryMatch>>>，每个 pattern 一项，顺序跟入参一致
    let results: Vec<Vec<Vec<QueryMatch>>> = tree_query::run_queries(&[
        "(if_statement) @if".into(),
        "(for_statement) @for".into(),
    ]);

    // 拿指定位置的 AST 节点。返回 Option<QueryMatch>。
    // 行 1-based，列 0-based。
    if let Some(node) = tree_query::node_at(10, 4) {
        // node.node_kind, node.text, ...
    }

    // 拿一段行范围内的所有命名顶层节点。返回 Vec<QueryMatch>。
    let nodes: Vec<QueryMatch> = tree_query::nodes_in_range(1, 50);

    vec![]
}
```

Query pattern 用 [tree-sitter 的 S 表达式 query 语言](https://tree-sitter.github.io/tree-sitter/syntax-highlighting/queries)。重复跑同一个 pattern 没额外开销。

每个 `QueryMatch` 包含：

- `capture_name` —— pattern 里的 `@名字`
- `node_kind` —— tree-sitter 节点类型（比如 `"function_definition"`）
- `text` —— 匹配到的源码原文
- `start_line` / `end_line` —— **1-based** 行号（跟 `FunctionInfo.start_line` / `ClassInfo.start_line` 一致）
- `start_col` / `end_col` —— **0-based** 字节列

> **Note**：SDK 里所有行号都是 1-based，列号都是 0-based 字节偏移。

### Project Query API（`project_query`）

跨文件分析（调用方、类型来源、其他文件的函数体）通过 `project_query` host 函数：

**调用图**：

```rust
// 这个函数有没有被本文件之外的人调过？
let unused = !project_query::is_called_externally(&fn_name, &input.path);

// 哪些文件引用了 `name`
let callers: Vec<String> = project_query::callers_of(&fn_name);

// 全项目跨文件调用计数：(caller_path, callee_path, count) 元组
let counts: Vec<(String, String, u32)> = project_query::cross_file_call_counts();
```

**符号定义所在**：

```rust
// 这个函数 / 类首次声明在哪个文件
let f_home: Option<String> = project_query::function_home(&fn_name);
let c_home: Option<String> = project_query::class_home(&class_name);

// 这个函数名对应的 (文件, FunctionInfo) 元组
let f: Option<(String, FunctionInfo)> = project_query::function_by_name(&fn_name);

// 哪个函数声明覆盖了这个 (line, col)？
// 行 1-based，列 0-based。返回最内层匹配（行范围最小的那个）。
if let Some(host_fn) = project_query::function_at(&input.path, line, col) {
    // host_fn.start_line / host_fn.end_line 都是 1-based
}
```

**类型来源 / 项目形态**：

```rust
// 项目里有没有声明这个名字
let is_local = project_query::is_project_type(&type_ref.name);

// 是不是真正的第三方依赖
// （External origin，且不是 stdlib，也不是 workspace 同级 crate）
let is_3p = project_query::is_third_party(&type_ref);

// Rust workspace 同级 crate 名（非 Rust workspace 时是空）
let siblings: Vec<String> = project_query::workspace_crate_names();

// 路径是否符合测试文件特征：__tests__/ / __mocks__/ / .test.ts / .spec.ts 等
if project_query::is_test_path(&input.path) { /* ... */ }

// 全项目分析过的文件总数
let n: u32 = project_query::file_count();
```

`function_at` 用来回答"这个位置（行列）归属于哪个函数声明"——配合 tree-query 用得多，query 命中一个位置之后想拿到包含它的函数。

### FunctionInfo 字段

```rust
pub struct FunctionInfo {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub name_col: u32,
    pub name_end_col: u32,
    pub line_count: u32,
    pub complexity: u32,
    pub parameter_count: u32,
    pub parameter_types: Vec<TypeRef>,
    pub parameter_names: Vec<String>,
    pub chain_depth: u32,
    pub switch_arms: u32,
    pub switch_arm_values: Vec<ArmValue>,
    pub external_refs: Vec<String>,
    pub is_delegating: bool,
    pub is_exported: bool,
    pub comment_lines: u32,
    pub referenced_fields: Vec<String>,
    pub null_check_fields: Vec<String>,
    pub switch_dispatch_target: Option<String>,
    pub optional_param_count: u32,
    pub called_functions: Vec<String>,
    pub cognitive_complexity: u32,
    pub body_hash: Option<String>,
    pub return_type: Option<TypeRef>,
}
```

### ClassInfo 字段

```rust
pub struct ClassInfo {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub name_col: u32,      // 0-based 列号
    pub name_end_col: u32,  // 0-based 结束列
    pub line_count: u32,
    pub method_count: u32,
    pub field_count: u32,
    pub field_names: Vec<String>,
    pub is_exported: bool,
    pub has_behavior: bool,
    pub is_interface: bool,
    pub parent_name: Option<String>,
    pub override_count: u32,
    pub self_call_count: u32,
    pub has_listener_field: bool,
    pub has_notify_method: bool,
}
```

## 读配置项

`.cha.toml` 里写的选项：

```toml
[plugins.my-plugin]
threshold = 10
label = "custom"
tags = ["a", "b"]
```

用 SDK 提供的取值宏：

```rust
use cha_plugin_sdk::{option_int, option_str, option_list_str};

let threshold = option_int!(&input.options, "threshold").unwrap_or(5);
let label     = option_str!(&input.options, "label").unwrap_or("default");
let tags      = option_list_str!(&input.options, "tags").unwrap_or(&[]);
```

可用的宏：`option_str!` / `option_int!` / `option_float!` / `option_bool!` / `option_list_str!` / `str_options!`。

## 编译

```bash
cha plugin build
```

它跑 `cargo build --target wasm32-wasip1 --release`，再用内嵌的 WASI adapter 把输出转成 WASM Component。结果是当前目录下的 `<名字>.wasm`。

> **Warning**：发布时**不要直接用 `cargo build`**。Cargo 出来的原始 `.wasm` 是 core module，不是 component——Cha host 加载不了。`cha plugin build` 包了一层 component 编码（用 `wasm-tools component new` + WASI adapter）。
>
> 开发期间为了调试可以用 `cargo build`，但**重新装之前**要再跑一遍 `cha plugin build`，否则 host 拿到的还是上一版。

### WASM 兼容性速查

插件跑在 `wasm32-wasip1` + WASI Reactor adapter 里。一些 Rust crate 在这个环境**就算能编**也不能用：

| Crate / API | 状态 | 备注 |
|---|---|---|
| `regex` | ❌ runtime panic | `Regex::new()` 在当前 host 配置下会失败。改手写字符扫描——常见模式大概 50 LOC，更安全 |
| `std::time::SystemTime::now()` | ❌ 不可靠 / panic | WASI clock 各 host 不一致。要"今天的日期"就在 `.cha.toml` 加一个 `today` 选项 |
| `serde_json` | ✅ 能用 | 体积大，但没坑 |
| `tree-sitter`（Rust crate 本身） | ❌ 别用 | 插件已经在 WASM 里跑了；要 query 调 host 的 `tree_query` |
| 文件系统 | ❌ 沙箱关 | `std::fs::read_to_string(&input.path)` 返回空。读源码用 `input.content` |
| `git` / 网络 | ❌ 沙箱关 | 没子进程、没 socket |

不确定的时候：依赖尽量精简、小模式手写不引 crate、外部状态（时间 / 配置）通过插件选项传进来。

## 安装

```bash
cha plugin install my_plugin.wasm        # 项目级：.cha/plugins/
cp my_plugin.wasm ~/.cha/plugins/        # 全局
```

每次 `cha analyze` 都会从这两个位置加载所有 `.wasm` 插件。

## 列出 / 卸载

```bash
cha plugin list                  # 显示已装插件 + 各自的 smell 名单
cha plugin remove my_plugin      # 用 .wasm 文件名（不带 .wasm 也行）
```

## 配置

插件装好就默认启用。在 `.cha.toml` 里关闭或调参：

```toml
[plugins.my-plugin]
enabled = false       # 或者保留默认 true，下面这一项是给插件传配置
threshold = 20
```

section 名要跟 `name()` 返回的字符串一致。

## 测试

`Cargo.toml` 里加：

```toml
[dev-dependencies]
cha-plugin-sdk = { git = "https://github.com/W-Mai/Cha", features = ["test-utils"] }
```

`test-utils` feature 没默认开，所以 `dev-dependencies` 单独写一行带 `features` 的引用。SDK 还没在 crates.io 上发布，所以走 `git`。

写测试——`source(language, code)` 给测试一个虚拟源文件：

```rust
#[cfg(test)]
mod tests {
    use cha_plugin_sdk::test_utils::WasmPluginTest;

    #[test]
    fn detects_issue() {
        WasmPluginTest::new()
            .source("typescript", "function todo_fix() {}")
            .assert_finding("my_smell_name");
    }

    #[test]
    fn no_false_positive() {
        WasmPluginTest::new()
            .source("typescript", "function processData() {}")
            .assert_no_finding();
    }

    #[test]
    fn respects_options() {
        WasmPluginTest::new()
            .source("typescript", r#"fetch("https://example.com");"#)
            .option("DOMAIN", "example.com")
            .assert_finding("hardcoded_string");
    }

    #[test]
    fn list_options_work() {
        WasmPluginTest::new()
            .source("typescript", "// REVIEW: needs second look")
            .option_list("extra_tags", &["REVIEW"])
            .assert_finding("extended_todo_tag");
    }
}
```

可用的选项设置：

- `.option(key, value)` —— 字符串
- `.option_list(key, &[values])` —— 字符串列表
- `.option_bool(key, true_or_false)`
- `.option_int(key, integer)`
- `.option_float(key, float)`

跑：

```bash
cha plugin build
cargo test
```

`cargo test` 时如果 `.wasm` 不存在，`WasmPluginTest` 会自动跑一次 `cha plugin build`。

### 断言 API

| 方法 | 作用 |
|------|------|
| `.assert_any_finding()` | 至少一条 finding |
| `.assert_no_finding()` | 没有任何 finding |
| `.assert_finding("name")` | 至少一条命中指定 smell name 的 finding |
| `.assert_no_finding_named("name")` | 没有命中指定 smell name 的 finding |
| `.findings()` | 返回 `Vec<Finding>`，给自定义断言用 |

## 示例插件

仓库 [`examples/`](https://github.com/W-Mai/Cha/tree/main/examples) 下有 4 个端到端示例：

- [`wasm-plugin-example`](https://github.com/W-Mai/Cha/tree/main/examples/wasm-plugin-example) —— 抓可疑函数名
- [`wasm-plugin-hardcoded`](https://github.com/W-Mai/Cha/tree/main/examples/wasm-plugin-hardcoded) —— 抓被配置驱动的硬编码字符串
- [`wasm-plugin-react-hooks`](https://github.com/W-Mai/Cha/tree/main/examples/wasm-plugin-react-hooks) —— React Hooks 规则
- [`wasm-plugin-todo-tracker`](https://github.com/W-Mai/Cha/tree/main/examples/wasm-plugin-todo-tracker) —— TODO/FIXME 跟踪器

## WIT 接口

想看 host 跟插件之间的契约长啥样，完整 WIT 在 [`wit/plugin.wit`](https://github.com/W-Mai/Cha/blob/main/wit/plugin.wit)。

```wit
world analyzer {
    use types.{analysis-input, finding};

    import tree-query;
    import project-query;

    export name: func() -> string;
    export version: func() -> string;       // 自动从 Cargo.toml 读
    export description: func() -> string;   // 自动从 Cargo.toml 读
    export authors: func() -> list<string>; // 自动从 Cargo.toml 读
    export smells: func() -> list<string>;  // 来自 PluginImpl::smells（默认空）
    export analyze: func(input: analysis-input) -> list<finding>;
}
```
