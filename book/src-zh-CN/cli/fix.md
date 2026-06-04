# fix

自动改简单问题。

**当前只支持一种**：`naming_convention` —— 把不符合 PascalCase 的类名改对。其他 smell 还得手动修。能修的范围由 `Plugin::try_fix` 接口决定，未来插件可以提供更多自动修复。

## 用法

```
cha fix [参数] [路径...]
```

## 示例

```bash
# 看会改什么，但不真改
cha fix src/ --dry-run

# 真改
cha fix src/

# 只针对工作区改动过的文件
cha fix --diff
```

改动是 AST 级的——只改标识符 token，字符串字面量和注释里的同名字符串不会被误伤。

## 参数

| 参数 | 默认 | 说明 |
|------|------|------|
| `--dry-run` | `false` | 只显示会改什么，不写文件 |
| `--diff` | `false` | 只处理工作区未提交的改动文件 |
| 路径 | `.` | 处理范围 |

## 参考

- [`cha analyze`](./analyze.md) —— 先看有哪些 finding
- [`naming_convention` smell 定义](../plugins/reference.md#naming) —— 触发条件
- [插件开发：try_fix](../plugins/development.md) —— 给自己的插件加自动修
