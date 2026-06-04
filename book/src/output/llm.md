# LLM Context

Markdown-shaped output meant to be pasted into a chat with Claude, ChatGPT, or any other AI coding assistant. No JSON envelope, no extra noise — just the findings, formatted so an LLM can act on them.

## Sample output

```text
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

- **Smell**: data_class
- **Category**: Dispensables
- **Severity**: Hint
- **Location**: cha-core/src/cache.rs:8-15 (`FileEntry`)
- **Problem**: Class `FileEntry` has 4 fields but no behavior methods, consider Move Method
- **Suggested refactorings**:
  - Move Method
  - Encapsulate Field

…

Please apply the suggested refactorings to improve code quality.
```

(Captured with `cha analyze --format llm cha-core/src/cache.rs`.)

## When to use it

- Pasting findings directly into a chat: "here's what Cha says about my code, please fix it."
- Feeding into an agentic coding tool (Claude Code, Cursor, Aider) that reads markdown well but stumbles on raw JSON.
- Generating a refactoring plan: each issue is a self-contained section with location, problem, and concrete suggestions.
- Sharing in chat / Slack / a doc — the markdown renders cleanly anywhere.

## Notes / Gotchas

- Output is plain markdown — no surrounding JSON, no escape characters. An LLM can quote it, edit it, or extract pieces without parsing.
- Empty runs produce the single line `No code smells detected.` instead of an empty document.
- For programmatic consumption, use [JSON](json.md) instead — this format is intentionally lossy: thresholds, risk scores, and exact column ranges in the upper line are stripped to keep the prose dense.
- Pair with `--top` and `--focus` to keep the paste short; LLMs handle 20 well-scoped issues better than 200 unfiltered ones.
- The trailing instructional sentence (`Please apply the suggested refactorings…`) is part of the output. Strip it if you're embedding the result inside a larger prompt.
