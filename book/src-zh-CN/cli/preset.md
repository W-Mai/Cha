# preset

看 Cha 内置的语言 profile 和严格度等级——每种语言开了哪些插件、关了哪些 smell、阈值调成了什么。看完再写自己的 `.cha.toml` 覆盖。

两个子命令：`list`（看哪些语言有 profile）/ `show <语言>`（看某语言的完整解析配置）。

## 用法

```
cha preset list
cha preset show <语言>
```

`<语言>` 可以是：`rust` / `typescript` / `python` / `go` / `c` / `cpp`。

## 示例

```bash
# 哪些语言有 profile
cha preset list

# C 的完整 profile
cha preset show c

# Rust 的（基本就是默认）
cha preset show rust
```

`cha preset show c` 会列出：

- 当前严格度系数
- 启用的插件清单
- 被 profile 禁用的插件 / smell
- profile 调高 / 调低的阈值

目前实际上只有 C / C++ 的 profile 真的修改默认值（procedural 语言不适用 OO 类规则）。其他语言的 profile 存在但暂时没改默认。

## 参数

| 子命令 | 参数 | 说明 |
|-------|------|------|
| `list` | — | 列所有有 profile 的语言 |
| `show` | 语言名 | 显示该语言的完整解析配置 |

## 参考

- [严格度与预设](../configuration/presets.md) —— 写自己的 profile 覆盖
- [配置](../configuration/overview.md)
