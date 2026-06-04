# pre-commit hook

提交前自动跑 cha——只检查这次改动的文件，warning 级别以上就拦下提交。

## 1. 装 [pre-commit](https://pre-commit.com)

```bash
pipx install pre-commit
# 或者
brew install pre-commit
```

## 2. 在仓库根加 `.pre-commit-config.yaml`

```yaml
repos:
  - repo: https://github.com/W-Mai/Cha
    rev: v1.19.0
    hooks:
      - id: cha-analyze
```

## 3. 装到 git hooks

```bash
pre-commit install
```

下次 `git commit` 时会先跑 `cha analyze --diff --fail-on warning`——只扫这次 staged 的 + 工作区改动的文件，碰到 warning 或 error 就阻止提交。

## 跑得太吵？

新仓库一接入大量 finding 拦提交，体验很差。三种缓解：

**调高 fail 门槛**：只拦 error，不管 warning。fork 一份 hook 配置：

```yaml
hooks:
  - id: cha-analyze
    entry: cha analyze --diff --fail-on error
```

**用 baseline**：先 `cha baseline` 拍快照，hook 只看 baseline 之外：

```yaml
hooks:
  - id: cha-analyze
    entry: cha analyze --diff --fail-on warning --baseline .cha/baseline.json
```

**完全跳过单次提交**：

```bash
git commit --no-verify
```

但**别**把 `--no-verify` 写成习惯——hook 拦不住的事 CI 会拦。

## 接下来

- [GitHub Actions 集成](github-actions.md) —— 即使 hook 被绕过，CI 也兜底
- [Baseline 工作流](../recipes/baseline.md) —— 老项目接入推荐流程
