# plugin

WASM 插件生命周期：脚手架 / 编译 / 安装 / 列出 / 删除。

写插件的细节在 **[插件开发](../plugins/development.md)** —— 那边有完整流程（WIT 接口、tree_query、project_query、option 读取、单测）。本页只是命令清单。

## 用法

```
cha plugin <子命令> [...]
```

## 子命令

| 子命令 | 用途 |
|-------|------|
| `new <名字>` | 脚手架一个新插件项目（cdylib + cha-plugin-sdk） |
| `build` | 跑 `cargo build --target wasm32-wasip1 --release`，再用 wasm-tools 转成 Component |
| `install <文件.wasm>` | 装到当前项目 `.cha/plugins/`（项目级） |
| `list` | 列已装插件 + 它们能产出哪些 smell |
| `remove <名字>` | 卸载（含 `.wasm` 后缀都行） |

## 示例

```bash
# 在当前空目录脚手架一个，或在父目录创建新子目录
mkdir my-rule && cd my-rule
cha plugin new my-rule

# 编译并转 Component
cha plugin build

# 装到本项目
cha plugin install my_rule.wasm

# 装全局（手动 cp）
cp my_rule.wasm ~/.cha/plugins/

# 看装了什么
cha plugin list

# 删掉
cha plugin remove my_rule
```

## 装哪儿

- 项目级：`.cha/plugins/`（跟着仓库走）
- 全局：`~/.cha/plugins/`（个人电脑）

`cha analyze` 每次都从这两个目录加载所有 `.wasm`。

## 参考

- [插件开发完整指南](../plugins/development.md)
- [内置插件清单](../plugins/reference.md)
- [配置：插件选项](../configuration/overview.md) —— 在 `.cha.toml` 里给插件传配置
