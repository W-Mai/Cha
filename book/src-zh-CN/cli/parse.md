# parse

把文件扔进 tree-sitter 解析，把它能看到的结构 dump 出来——函数、类、import、注释、跨文件引用。debug 插件、确认 Cha 是否正确识别某段代码时用。

## 用法

```
cha parse [路径...]
```

## 示例

```bash
# dump 当前目录
cha parse

# 指定文件
cha parse src/main.rs

# 指定多个路径
cha parse src/ tests/
```

输出包含每个函数 / 类的位置、行数、复杂度、参数列表、外部引用等——基本是 Cha 内部 model 的可读版。

## 参数

| 参数 | 默认 | 说明 |
|------|------|------|
| 路径 | `.` | 要解析的文件或目录（默认当前目录） |

## 参考

- [`cha analyze`](./analyze.md) —— 解析之上跑插件检测
- [插件开发](../plugins/development.md) —— 写自定义插件时会接触到这些 model 字段
