# 写一条 smell

下面拿 `MiddleManAnalyzer`（[源码](https://github.com/W-Mai/Cha/blob/main/cha-core/src/plugins/middle_man.rs)，64 行）作为例子走一遍。一共四步：写 analyzer、选 category、注册、补测试和文档。

这一篇是写**内置 smell**（跟 `cha-core` 一起编译进 cha 的那种）。如果你只想写一条项目专属的、不进主仓的检测器，写 WASM 插件 —— 见 [50 行写一个插件](../recipes/custom-plugin-50loc.md)。

## 第 1 步：写 analyzer

新建 `cha-core/src/plugins/<your_smell>.rs`：

```rust
use crate::{AnalysisContext, Finding, Location, Plugin, Severity, SmellCategory};

pub struct MiddleManAnalyzer {
    pub min_methods: usize,
    pub delegation_ratio: f64,
}

impl Default for MiddleManAnalyzer {
    fn default() -> Self {
        Self {
            min_methods: 3,
            delegation_ratio: 0.5,
        }
    }
}

impl Plugin for MiddleManAnalyzer {
    fn name(&self) -> &str { "middle_man" }
    fn smells(&self) -> Vec<String> { vec!["middle_man".into()] }
    fn description(&self) -> &str { "Class that only delegates to others" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        ctx.model.classes.iter()
            .filter(|c| {
                c.method_count >= self.min_methods
                    && c.delegating_method_count > 0
                    && (c.delegating_method_count as f64 / c.method_count as f64)
                        >= self.delegation_ratio
            })
            .map(|c| Finding {
                smell_name: "middle_man".into(),
                category: SmellCategory::Couplers,
                severity: Severity::Hint,
                location: Location {
                    path: ctx.file.path.clone(),
                    start_line: c.start_line,
                    start_col: c.name_col,
                    end_line: c.start_line,
                    end_col: c.name_end_col,
                    name: Some(c.name.clone()),
                },
                message: format!(
                    "Class `{}` delegates {}/{} methods, acting as a middle man",
                    c.name, c.delegating_method_count, c.method_count
                ),
                suggested_refactorings: vec!["Remove Middle Man".into()],
                actual_value: Some(c.delegating_method_count as f64 / c.method_count as f64),
                threshold: Some(self.delegation_ratio),
                risk_score: None,
            })
            .collect()
    }
}
```

约定：

- **struct 字段就是阈值。** `analyze()` 里不要出现魔法数字。默认值放 `Default`。
- **`name()` 是插件标识。** 在 `--plugin <name>`、`[plugins.<name>]` config、`// cha:ignore <name>` 里都是它。
- **`smells()` 列出这个插件会产出的所有 smell 名。** 大多数插件只有一条 smell 跟 `name()` 同名；少数会产出多条（比如 `length` 一个插件出 `long_method` / `large_class` / `large_file`）。
- **严重度**：`Severity::Hint` 给纯风格层面的发现，`Warning` 给确实会伤害可读性 / 正确性的，`Error` 给 CI 应该拒绝的。
- **`actual_value` 和 `threshold` 是数值字段**，message 文案和 `--explain` 都会用上。只要 smell 有数值指标就填。

## 第 2 步：选 `SmellCategory`

类别决定 CLI 输出、JSON report 和 `--focus` 的分组。按 smell 真实形态选：

| Category | 装什么 |
|---|---|
| `Bloaters` | 长出来的代码（`long_method`、`god_class`、`complexity`）。 |
| `Couplers` | 模块之间耦合过紧（`coupling`、`feature_envy`、`middle_man`）。 |
| `OOAbusers` | 面向对象构造用错地方（`switch_statement`、`refused_bequest`、`design_pattern`）。 |
| `ChangePreventers` | 一处修改逼迫多处修改（`shotgun_surgery`、`divergent_change`）。 |
| `Dispensables` | 删了不影响功能的（`dead_code`、`duplicate_code`、`lazy_class`）。 |
| `Security` | 危险调用 / 泄露的密钥（`hardcoded_secret`、`unsafe_api`）。 |

如果一条 smell 看着横跨两个 category，选更具体的那个——`SmellCategory` 是个枚举，一条 finding 只能挂一个。

## 第 3 步：注册

改 [`cha-core/src/plugins/mod.rs`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/plugins/mod.rs)：

```rust
mod middle_man;
pub use middle_man::MiddleManAnalyzer;
```

改 [`cha-core/src/registry.rs`](https://github.com/W-Mai/Cha/blob/main/cha-core/src/registry.rs)，找到对应 category 的 `register_*_plugins` 函数，加上：

```rust
register_if_enabled(plugins, config, "middle_man", || {
    let mut p = MiddleManAnalyzer::default();
    apply_usize(config, "middle_man", "min_methods", &mut p.min_methods);
    apply_f64(config, "middle_man", "delegation_ratio", &mut p.delegation_ratio);
    Box::new(p)
});
```

`apply_*` 把 `.cha.toml` 里 `[plugins.middle_man]` 的配置覆盖到默认阈值上。analyzer 没有可配字段就不写 `apply_*`。

`register_if_enabled` 自己会处理 `[plugins.middle_man]` 里 `enabled = false` 的情况，你不用管。

## 第 4 步：测试 + 文档

新建 `cha-core/src/plugins/<your_smell>_tests.rs`（或者塞进现有测试文件）。模板：

```rust
#[test]
fn fires_on_middle_man() {
    let src = r#"
        class Wrapper {
            fn foo(&self) { self.inner.foo() }
            fn bar(&self) { self.inner.bar() }
            fn baz(&self) { self.inner.baz() }
        }
    "#;
    let findings = analyze_with(MiddleManAnalyzer::default(), "rust", src);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].smell_name, "middle_man");
}

#[test]
fn does_not_fire_below_threshold() {
    // ... 2 个委托方法，默认 min_methods=3
}
```

测试粒度要细：一条测"该报的有报"，一条测"低于阈值不报"，每个值得测的边界一条。`cha-core/tests/fixtures/` 下的 fixture 测试是用来测跨插件交互的，单插件单元测试用内联源码字符串更清楚。

然后改三处文档：

1. **README.md 插件表** —— 在对应 `SmellCategory` 段加一行，写 smell 名、默认阈值、严重度。**README.zh-CN.md** 同步加。
2. **docs/plugins.md** —— 写完整描述，附一个能触发它的代码示例。**docs/plugins.zh-CN.md** 同步加。
3. **CHANGELOG.md** 的 `[Unreleased]` —— 在 "Added" 下加一行。

book 里的插件参考页是通过 `{{#include}}` 引 `docs/plugins.md` 的，不用单独改。

## 自验

```bash
cargo xtask ci   # 跑 build + test + lint + analyze
```

接着 dogfood —— 用新插件分析 cha 自己的代码：

```bash
cargo run -- analyze --plugin middle_man cha-core/
```

如果它在 cha 自己的代码上报 finding，得做判断：是真问题（去修 cha），还是误报（收紧 analyzer）。

## See also

- [`Plugin` trait](https://github.com/W-Mai/Cha/blob/main/cha-core/src/plugin.rs)
- [`SmellCategory` 枚举](https://github.com/W-Mai/Cha/blob/main/cha-core/src/model.rs)
- [架构](./architecture.md) —— 看这条插件在数据流里的位置。
- [插件开发](../plugins/development.md) —— 写 WASM 插件（不进主仓）的话看这里。
