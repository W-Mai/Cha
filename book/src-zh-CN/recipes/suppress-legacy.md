# 遗留代码豁免

半路接入 cha。第一次 `cha analyze` 报了 200 条 finding，一半是早就在那儿的、跟现在团队无关的代码。CI 要绿，但又不能假装代码没问题。

按优先级三件武器：**baseline** 表示"以后再说"，**行内指令** 表示"这一处是合理的"，**配置 exclude** 表示"这个路径根本不要看"。

## 1. 先打 baseline

把当前所有 finding 拍个快照，之后只对新 finding 失败。

```bash
cha baseline
git add .cha/baseline.json && git commit -m "Cha baseline at adoption"
```

CI 用：

```bash
cha analyze --baseline .cha/baseline.json --fail-on warning
```

引入新 finding 的 PR 会挂掉，旧的 finding 静默放过。baseline 文件本身是一组指纹（每条 finding 的稳定标识），体积小、commit 干净、技术债（debt）还掉时 diff 也读得清。

完整流程：[Baseline 工作流](./baseline.md)。

## 2. 行内指令处理特定 item

某个 item 真的有理由违规（200 行的状态机、构造函数实在需要 9 个参数）：

```rust
// cha:ignore long_method
fn dispatch_state_machine(&mut self, event: Event) -> State {
    match self.current {
        // ... 200 行有正当理由的代码
    }
}
```

```python
# cha:ignore long_parameter_list
def __init__(self, host, port, user, password, db, ssl_cert, retry, timeout):
    ...
```

对下一个 item 屏蔽一条、多条或全部：

```rust
// cha:ignore                        — 屏蔽所有
// cha:ignore long_method            — 屏蔽一条
// cha:ignore long_method,complexity — 屏蔽多条
// cha:set long_method=200           — 单独把这个 item 的阈值放宽
```

行内指令**不会被写进 baseline 文件**——它就是写在源码里的明确决定，做不做 baseline 都在那儿。当你希望豁免在 code review 里看得见时，用它。

完整语法见 [行内指令](../configuration/inline-directives.md)。

## 3. 配置 exclude 整个路径

某些文件 cha 根本不该看——生成代码、第三方 vendoring、测试 fixture：

```toml
# .cha.toml
exclude = [
    "vendor/**",
    "src/generated/**",
    "tests/fixtures/**",
    "node_modules/**",   # 文件遍历器本来就尊重 .gitignore，这条通常多余
]
```

模式是 glob。`**` 匹配任意层。被排除的路径根本不会被解析——比"跑了再屏蔽"便宜。

## 决策表

| 处境 | 用什么 |
|---|---|
| 满地都是历史 finding，今天就要绿 CI | baseline |
| 一个文件里一条特别顽固的 finding | 行内 `cha:ignore` |
| 一个文件需要单独的阈值 | 行内 `cha:set` |
| 整个目录就不该被分析 | 配置 `exclude` |

可以叠加。baseline、行内、exclude 是三层独立机制，cha 的处理顺序是 `exclude` → 分析 → `cha:ignore` / `cha:set` → baseline 过滤。一条 finding 要四层全放过才会冒出来。

## 还债

baseline 不是"永久无视"。隔段时间：

```bash
cha baseline                       # 重新生成，捕获当前状态
git diff .cha/baseline.json        # 看少了哪些
```

如果 `git diff` 显示有条目消失，说明 debt 还掉了。如果反而出现新条目，那 CI 里的 `--baseline` 没生效，去查配置。

## See also

- [Baseline 工作流](./baseline.md)
- [行内指令](../configuration/inline-directives.md)
- [`cha baseline`](../cli/baseline.md)
