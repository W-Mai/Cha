# HTML

自包含 HTML 报告——单个 `.html` 文件，CSS/JS 内嵌，不依赖外部资源。可以直接发邮件、放静态站、当 PR 评论附件。

## 用法

```bash
cha analyze --format html --output report.html
```

`--output` 是必给的——HTML 报告太大，不会往 stdout 喷。

## 报告内容

- 顶部 summary：总 finding 数、按严重度 / 类目分布、估算的技术债分钟、每个文件的 grade（A-F）
- 按文件展开的 finding 列表，带源代码片段（finding 触发行高亮）
- 按 smell 名分组的索引（点 smell 名跳到所有触发位置）
- 按类目过滤的标签

## 适用场景

- **每周 / 每月报告**：CI 定时任务生成，邮件 / Slack 链接发出去
- **PR 大改动评估**：本地跑一遍 HTML 给 reviewer，比让他们 checkout 看 terminal 直接
- **当快照对比**：今天的报告 vs 上月的，看 grade 变化
- **客户 / 上级**：技术债不会给非工程师看 JSON，HTML 一目了然

## 备注

- 报告默认是英文界面（标签、按钮文字）。后续可能加多语言
- 文件大小取决于 finding 数量——千百条 finding 的项目报告可能几 MB
- 不要把含有公司源码片段的 HTML 公开放——里面会内嵌触发行附近的代码
