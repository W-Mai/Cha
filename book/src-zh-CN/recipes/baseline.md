# Baseline 工作流

baseline 是某一时刻所有 finding 的快照。`cha analyze --baseline <path>` 会把这份快照里的 finding 过滤掉，CI 只对快照之后才出现的 finding 失败。

这一篇是日常节奏：怎么生成、怎么对比、什么时候刷。

## 生成

在主干上跑一次，时机选在团队明确"这些都先放着"的那一刻：

```bash
cha baseline
```

默认落到 `.cha/baseline.json`。要换位置用 `-o`：

```bash
cha baseline -o .cha/legacy-2026-Q1.json
```

文件本身是一组指纹——`(路径, smell 名, 归一化后的位置)`。位置之所以"归一化"是因为行号小幅移动时仍要能匹配（多加几行注释不应该让 finding "复活"）。文件体积小、diff 友好，commit 进仓。

```bash
git add .cha/baseline.json
git commit -m "Cha baseline at adoption"
```

## CI 里使用

```bash
cha analyze --baseline .cha/baseline.json --fail-on warning
```

baseline 里的指纹静默放过，其他 finding 正常报。新 finding 出现在改动行 → CI 失败；老 finding 继承下来 → CI 安静通过。

GitHub Actions step：

```yaml
- name: cha
  run: cha analyze --baseline .cha/baseline.json --fail-on warning
```

## 对比

过几周看一下变了什么：

```bash
cha baseline -o /tmp/now.json
diff -u .cha/baseline.json /tmp/now.json | less
```

`-` 行（baseline 里有、现在没有）表示这条 finding 不见了——技术债还掉了。`+` 行（现在有、baseline 里没有）正常情况下不会出现；如果出现，说明 CI 里的 `--baseline` 没生效，去查配置。

## 刷新

团队还了足够多的 debt、原 baseline 大半已经过时之后，重新生成：

```bash
cha baseline                 # 覆盖 .cha/baseline.json
git diff .cha/baseline.json  # 看少了哪些
git commit -am "Refresh Cha baseline (-32 entries)"
```

刷新可以按节奏（每季度是常见选择），或者在大重构落地后做一次。commit message 里写清楚少了多少条——这是个能在团队回顾会上拿出来讲的真实数字。

## 多 package

monorepo 里每个 package 一份 baseline：

```bash
for pkg in packages/*/; do
  ( cd "$pkg" && cha baseline )
done
```

每份 `.cha/baseline.json` 跟对应 package 放一起，CI 按 package 跑。详见 [Monorepo CI](./monorepo-ci.md)。

## baseline **不**解决的事

- **规则错了** —— 如果某条 smell 一直在你不关心的代码上炸出来，别把它埋进 baseline。要么在 `.cha.toml` 里调阈值，要么在那一处用 [行内指令](../configuration/inline-directives.md)，要么把整条插件 `enabled = false`。
- **指纹漂移（drift）** —— baseline 屏蔽的是已有 finding 的指纹。新写的代码即使出了形态完全相同的 finding，**只要指纹不一字不差地命中现有条目**，照样会报。换句话说 baseline 不是"silence smell × file"那种粗粒度规则，它认的是具体那一处。

## See also

- [`cha baseline`](../cli/baseline.md)
- [遗留代码豁免](./suppress-legacy.md)
- [Monorepo CI](./monorepo-ci.md)
