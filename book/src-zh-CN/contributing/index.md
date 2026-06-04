# 贡献

发 PR 之前可能用得上的三页内容。都不是必读——项目本身够小，靠读代码也能摸清楚——但每一页都能帮你少跑一轮 review。

| 页面 | 什么时候读 |
|---|---|
| [架构](./architecture.md) | 改动跨 crate 边界（`cha-core` ↔ `cha-parser`、插件 runtime、LSP）时。 |
| [写一条 smell](./writing-a-smell.md) | 加一条新内置检测器（落到 `cha-core/src/plugins/`）。 |
| [发版](./releasing.md) | 切一个新版本。流程基本自动化，但有几条不变量要守。 |

本地开发：

```bash
cargo xtask ci          # 把 CI 跑的全跑一遍
cargo xtask test        # 只跑测试
cargo xtask lint        # clippy + fmt
cargo xtask analyze     # cha 自分析（每种输出格式都过一遍）
```

代码评审风格：commit 切小、关注点切开。bug 修复一个 commit、refactor 下一个、文档再下一个。[release 流程](./releasing.md)（`cargo xtask release`）不做 squash，干净的历史靠你自己。
