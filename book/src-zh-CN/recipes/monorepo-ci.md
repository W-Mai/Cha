# Monorepo CI

一个仓里好几个 package，一个 PR 通常只改其中一两个。但裸跑 `cha analyze` 会把整棵树都走一遍，剩下那些没改的也算你头上。

下面两种做法，常常配在一起用。

## 做法 1：只看动过的文件

`cha analyze --diff` 跑工作区里改动过的文件。在 PR 流水线里，把 PR diff 喂给它：

```bash
# 本地：相对工作区改动过的文件
cha analyze --diff

# CI：把 PR diff 用管道送进来
gh pr diff "$PR_NUMBER" | cha analyze --stdin-diff --fail-on warning
```

`--stdin-diff` 接受标准 unified-diff 格式。同一份 `.cha.toml` 照样生效，只是文件列表收窄了。

GitHub Actions 里这么写：

```yaml
- name: cha (PR diff)
  run: |
    gh pr diff ${{ github.event.pull_request.number }} \
      | cha analyze --stdin-diff --fail-on warning
  env:
    GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

push 到主干的 event 走全量：

```yaml
- name: cha (push)
  run: cha analyze --fail-on warning
```

## 做法 2：每个 package 一份配置

monorepo 里如果是 `packages/api`、`packages/web`、`packages/shared` 这种结构，给每个 package 自己的 `.cha.toml`，按 package 跑：

```bash
for pkg in packages/*/; do
  ( cd "$pkg" && cha analyze --fail-on warning ) || exit 1
done
```

每份 `.cha.toml` 都是独立的——这是有意为之。共享库可以严一点（`max_function_lines` 调小），实验性的 package 可以放宽 `complexity`。cha 不会让子目录的 `.cha.toml` 自动继承父目录的设置——因为 monorepo 里不同 package 本来就应该有不同形状的约束。

## 组合：每包一份 baseline + PR 走 diff

能撑住规模的常见配方：

1. 每个 package 都有自己的 `.cha/baseline.json`，接入 cha 那天生成一次。
2. PR 同时用 `--baseline` 和 `--stdin-diff`：

   ```bash
   cha analyze --stdin-diff \
     --baseline .cha/baseline.json \
     --fail-on warning < diff.patch
   ```
3. push 到主干的全量分析仍然带 `--baseline`——这样 PR 没动到的文件如果跑出新 finding（比如有人手动改了什么）也能被发现。

旧 finding 被 baseline 屏蔽。新 finding 在改动行上挂掉 CI。diff 之外的旧文件不再处理。

## 缓存

cha 把解析好的 AST 和 finding 结果缓在 `.cha/cache/`。在 CI 里把这个目录缓起来：

```yaml
- uses: actions/cache@v4
  with:
    path: .cha/cache
    key: cha-${{ hashFiles('**/Cargo.lock', '**/package-lock.json', '**/go.sum') }}
```

cache key 跟着依赖锁文件走就够细了，再细也得不偿失。

## See also

- [Baseline 工作流](./baseline.md)
- [遗留代码豁免](./suppress-legacy.md)
- [`cha analyze`](../cli/analyze.md)
