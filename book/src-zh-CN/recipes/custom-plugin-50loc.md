# 50 行写一个插件

具体例子：检测函数名里带 `tmp`、`temp`、`xxx` 的函数。这种项目专属的命名洁癖，没有内置 smell 能覆盖。

`cha plugin new` 生成的脚手架比这里真正需要的多一些。这一篇砍到 50 行 Rust 加一份 `Cargo.toml`。完整的开发流程参考去 [插件开发](../plugins/development.md)。

## 起骨架

```bash
cha plugin new no-tmp-names
cd no-tmp-names
```

`Cargo.toml`（scaffold 自动写的版本能跑，关键内容是这些）：

```toml
[package]
name = "no-tmp-names"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
cha-plugin-sdk = { git = "https://github.com/W-Mai/Cha" }
wit-bindgen = "0.55"
```

## 插件主体

插件实现 `PluginImpl` trait：拿到 `AnalysisInput`（一个文件的解析结果 + 配置），返回 `Vec<Finding>`。

`src/lib.rs`：

```rust
use cha_plugin_sdk::{plugin, AnalysisInput, Finding, PluginImpl, Severity};

plugin!(NoTmpNames);

struct NoTmpNames;

const FORBIDDEN: &[&str] = &["tmp", "temp", "xxx"];

impl PluginImpl for NoTmpNames {
    fn name() -> String {
        "no-tmp-names".into()
    }

    fn smells() -> Vec<String> {
        vec!["tmp_named_function".into()]
    }

    fn analyze(input: AnalysisInput) -> Vec<Finding> {
        let mut findings = Vec::new();
        // input.model.functions 是这个文件里所有函数的解析结果；每个 f 是
        // FunctionInfo，带函数名、行号、参数、复杂度等。完整字段表见
        // [插件开发](../plugins/development.md#functioninfo-字段)。
        for f in &input.model.functions {
            let lower = f.name.to_lowercase();
            if FORBIDDEN.iter().any(|bad| lower.contains(bad)) {
                findings.push(Finding {
                    smell: "tmp_named_function".into(),
                    severity: Severity::Hint,
                    line: f.start_line,
                    column: f.name_col + 1,           // 1-based
                    end_line: Some(f.start_line),
                    end_column: Some(f.name_end_col + 1),
                    message: format!(
                        "函数 `{}` 的名字像临时占位——合并前给它一个像样的名字。",
                        f.name
                    ),
                    suggestion: None,
                });
            }
        }
        findings
    }
}
```

整个插件就是 `PluginImpl` trait 三个方法 + 一个循环。没状态、没 async、没 `Result` 仪式。

## 编译并安装

```bash
cha plugin build              # 产物在 target/wasm32-wasip2/release/no_tmp_names.wasm
cha plugin install no_tmp_names.wasm
```

`install` 把 `.wasm` 拷到 `.cha/plugins/`（项目级）。要全局装就加 `--global`，落到 `~/.cha/plugins/`。

## 跑起来

```bash
cha analyze --plugin no-tmp-names src/
```

迭代时只跑这一个插件；下一次 `cha analyze`（不带 `--plugin`）会把所有内置插件 + 你新写的这个一起跑。

## 改一改再跑

改完 `src/lib.rs` 后：

```bash
cha plugin build
cha plugin install no_tmp_names.wasm    # 覆盖上一份 .wasm
cha analyze --plugin no-tmp-names src/
```

cha 会缓存解析结果加速重复分析。装了新 `.wasm` 之后，凡是这个插件碰过的文件，缓存都会自动作废，不用手动清。

## `analyze` 里能拿到什么

`AnalysisInput` 暴露：

- `input.path` —— 当前正在分析的文件路径。
- `input.model` —— [SourceModel](../plugins/development.md#functioninfo-字段)，里面是解析好的函数、类、imports、注释。
- `input.options` —— `.cha.toml` 里 `[plugins.no-tmp-names]` 的值。

想跨文件查询（谁调用了这个函数、这个类型从哪来、本项目一共几个文件）或者写 tree-sitter S 表达式 query，看 [插件开发](../plugins/development.md)。

## See also

- [插件开发](../plugins/development.md) —— 完整参考。
- [`examples/`](https://github.com/W-Mai/Cha/tree/main/examples) —— 四个端到端的示例插件，含 TODO tracker 和 React hooks linter。
- [`cha plugin`](../cli/plugin.md)
