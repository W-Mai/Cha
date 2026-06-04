# LLM 上下文

专门给 AI / LLM 当上下文用的格式。**紧凑的 markdown**，没 JSON 包装，不带元数据噪音——直接复制粘进 Claude / ChatGPT / Cursor 等就能让 AI 帮分析、解释、提修复方案。

## 样例输出

```markdown
# Code Smell Analysis

## Issue 1

- **Smell**: lazy_class
- **Category**: Dispensables
- **Severity**: Hint
- **Location**: cha-core/src/cache.rs:8:7-8:16 (`FileEntry`)
- **Problem**: Class `FileEntry` has only 0 method(s) and 8 lines, consider Inline Class
- **Suggested refactorings**:
  - Inline Class

## Issue 2

- **Smell**: lazy_class
- **Category**: Dispensables
…
```

## 适用场景

- **Code review 时让 AI 给修复建议**：`cha analyze foo.rs --format llm | pbcopy`，粘进 Claude 让它一条条改
- **批量重构计划**：把整个文件 / 模块的 finding 喂给 AI，让它估算重构工作量
- **学习模式**：AI 接手解释每条 smell 为什么是问题、Refactoring Guru 上的对应章节
- **塞进 prompt context**：MCP 工具或自动化 agent 把 finding 当输入

## 跟 JSON 的区别

| 维度 | `--format json` | `--format llm` |
|------|----------------|---------------|
| 体积 | 较大（JSON 字段名 + 嵌套） | 紧凑（每个 issue ~6 行） |
| 机器友好 | ✅ | ❌（结构化提取困难） |
| LLM 友好 | 凑合（要让 AI 解析 JSON） | ✅（markdown 是 LLM 母语） |
| 适合 jq | ✅ | ❌ |

要给 AI 看 → `llm`。要给脚本看 → `json`。

## 备注

- 输出是英文的——LLM 现阶段对英文表述的 smell 名字识别更准。如果要中文 prompt，自己包一层告诉 AI "这是英文报告，请用中文回复"
- 没有 schema——这是设计如此，给 LLM 看不需要 schema
