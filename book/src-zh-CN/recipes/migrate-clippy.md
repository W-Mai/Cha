# 从 clippy 迁移

clippy 和 cha 回答的不是同一个问题。clippy 是 Rust 语言级 lint：它管 borrow check、idiom、生命周期陷阱。cha 看的是设计层面的问题——函数太长、一个类管太多事、一个模块太爱碰另一个模块的内部、依赖关系成了枢纽、跨层调用违规。这些在 cha 里有专门的 smell 名（`long_method`、`god_class`、`feature_envy`、`hub_like_dependency`、`layer_violation`），完整列表见 [Smell 列表](../plugins/reference.md)。

不要用 cha 替掉 clippy，两个一起跑。下面是接入时常见的两个摩擦点。

## 1. 并排跑就行

cha 不读 `cargo clippy` 的输出，两边连配置和 lockfile 都不共享。在你已有的流程里加一行：

```bash
# 已有
cargo clippy --all-targets -- -D warnings

# 新增
cha analyze --fail-on warning
```

CI 里两个独立 step。clippy 在 borrow check 上挂了，cha 就不必跑；反过来，cha 抓到设计问题时，clippy 也已经过了。

## 2. 调一下 Rust 项目的阈值

cha 的默认阈值跟语言无关。Rust 代码里通常要松一两个：

```toml
# .cha.toml
[plugins.length]
max_function_lines = 60   # Rust 的签名 + match 分支吃行数挺快

[plugins.complexity]
warn_threshold = 12
error_threshold = 24      # match 分支多的 Rust 代码，error 调到 24 比较合理
```

更靠谱的做法是先跑 `cha calibrate`（用项目自己的统计分布给你算阈值），看看实际的 P90 / P95 是多少，再决定是用那批数还是贴着默认。详见 [给你的项目校准阈值](./calibrate.md)。

## 3. clippy lint 跟 cha smell 怎么对应

绝大多数 clippy lint 在 cha 里没对应物，反过来也一样。少数有重叠：

| clippy lint | cha smell | 备注 |
|---|---|---|
| `too_many_arguments` | `long_parameter_list` | clippy 默认 7，cha 默认 5。 |
| `cognitive_complexity` | `cognitive_complexity` | 两边算的是同一个指标（SonarSource 提出的认知复杂度），阈值各管各的。 |
| `large_stack_arrays` | — | 栈大小分析不在 cha 范围内。 |
| `mod_module_files` | — | 风格问题，cha 不管。 |

两边都有的那几条，一般做法是 clippy 那条留着（clippy 看的是单个函数内部），让 cha 看跨函数 / 跨文件的关系。

## 4. 屏蔽生成代码的噪音

cha 不读 `#[allow(...)]`。即使生成代码在 clippy 那边贴了 `#[allow]`，cha 这边照样会报。两种处理方式，把路径写进 `exclude`：

```toml
exclude = ["src/generated/**", "build/**"]
```

或者在出问题的那个 item 顶上贴行内指令：

```rust
// cha:ignore
fn handler_generated_by_macro() { /* ... */ }
```

详见 [行内指令](../configuration/inline-directives.md)。

## See also

- [配置概览](../configuration/overview.md)
- [给你的项目校准阈值](./calibrate.md)
- [遗留代码豁免](./suppress-legacy.md)
