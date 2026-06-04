# init / schema

两个相关命令——一个生成默认配置，一个打印 finding 的 JSON Schema。

## `cha init`

在当前目录写一份默认 `.cha.toml`：

```bash
cha init
```

生成的文件已经有所有常用插件的默认阈值注释，改起来直接动数字就行。已存在 `.cha.toml` 不会覆盖。

模板内容跟 [`/Users/w-mai/Projects/Cha/static/default.cha.toml`](https://github.com/W-Mai/Cha/blob/main/static/default.cha.toml) 完全一致。

## `cha schema`

打印 Cha 输出 JSON 的 schema（[Draft 2020-12](https://json-schema.org/draft/2020-12) 格式）：

```bash
# 看 schema
cha schema

# 拉成本地文件给工具用
cha schema > findings.schema.json
```

`cha analyze --format json` 出来的所有字段都遵循这个 schema。配合 IDE 的 JSON Schema 支持，能在写 `.cha.toml` 或处理 finding 数据时拿到补全和校验。

JSON Schema 的完整字段说明见 **[JSON Schema 参考](../reference/json-schema.md)**。

## 参考

- [配置概览](../configuration/overview.md) —— 拿到 `.cha.toml` 之后的下一步
- [JSON 输出格式](../output/json.md) —— schema 描述的数据长什么样
- [`cha analyze --format json`](./analyze.md)
