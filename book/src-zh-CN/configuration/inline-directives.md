# 行内指令

在源码里写一行注释——放在函数 / 类前面（或同一行），就能屏蔽这一项的 findings、或者临时给它放宽阈值。

## `cha:ignore` —— 屏蔽 findings

| 写法 | 效果 |
|------|------|
| `// cha:ignore` | 下一项的所有规则都关 |
| `// cha:ignore <名>` | 只关一条 smell |
| `// cha:ignore <a>,<b>` | 关多条（逗号分隔） |

`<名>` 是 **smell 名**（CLI 输出里看到的，比如 `long_method` / `high_complexity` / `switch_statement`），**不是插件名**。`length` 一个插件就出三种 smell，要写具体的。

## `cha:set` —— 改阈值

| 写法 | 效果 |
|------|------|
| `// cha:set <smell>=<n>` | 把这条 smell 在这一项的阈值临时调到 `<n>` |
| `// cha:set threshold=<n>` | 把这一项所有"基于阈值"的 smell 阈值都调到 `<n>` |

`<n>` 是浮点数。如果实际测出来的值还是超过新阈值，finding 还会报。

`cha:set` 只对**有 `actual_value` / `threshold` 数值字段的 smell** 有效。布尔型 smell（比如 `inappropriate_intimacy`）忽略 `cha:set`——这种用 `cha:ignore` 关。

## 注释样式

四种都支持：`//`（Rust / TypeScript / Go / C / C++）、`#`（Python）、`--`（Lua / SQL）、`/* … */` 块注释：

```rust
// cha:ignore long_method
```

```python
# cha:ignore long_method
```

```c
/* cha:ignore long_method */
```

指令必须是**整行的开头**（去掉空白和注释标记后）。代码尾部的 trailing 注释里写 `cha:ignore` 不解析。

## 覆盖范围

指令对一条 finding 生效，要满足下面任一条件：

1. 指令跟 finding 在同一行，**或者**
2. 指令在 finding 起始行的**前 2 行内**

可以叠多条：

```rust
// cha:ignore long_method
// cha:set high_complexity=25
fn complicated_but_acknowledged() {
    // …
}
```

间隔超过 2 行就不生效。

## 例子

### 关一条规则

```rust
// cha:ignore long_method
fn render_template(/* … */) -> String {
    // 200 行模板生成器，知道，故意的
}
```

### 关多条

```typescript
// cha:ignore long_method,high_complexity
function migrateLegacyShape(input: unknown) {
  // …
}
```

### 给单条 smell 抬阈值

```rust
// cha:set long_method=120
fn parse_protocol_frame(buf: &[u8]) -> Frame {
    // 95 行 —— 超过默认 50 行，但在我们 120 行预算内
}
```

### 整体抬阈值

```python
# cha:set threshold=200
def state_machine_step(event):
    # 又长又分支多，故意的，长度 / 复杂度都别警告
    ...
```

抬完之后如果 `actual_value < threshold`，finding 就被丢掉；超不过的依然会报。
