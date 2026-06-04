# 烹饪书

每一篇都从一个具体处境讲起，落到一组能直接抄走的命令或配置。

| Recipe | 什么时候读它 |
|---|---|
| [从 clippy 迁移](./migrate-clippy.md) | Rust 项目原本跑 clippy，现在想加上 Cha 一起跑或者替掉。 |
| [Monorepo CI](./monorepo-ci.md) | 一个仓多个 package，PR 通常只动其中一两个。 |
| [遗留代码豁免](./suppress-legacy.md) | 半路接入 Cha，CI 一上来就被一堆历史 finding 淹了。 |
| [50 行写一个插件](./custom-plugin-50loc.md) | 想要一个项目专属的检测器，今天就要。 |
| [给你的项目校准阈值](./calibrate.md) | 默认阈值要么太严要么太松。 |
| [Baseline 工作流](./baseline.md) | baseline 文件的日常节奏：生成、对比、刷新。 |

如果你刚开始用 Cha，先去 [命令行快速开始](../quick-start/cli.md) 让 `cha analyze` 能跑起来，再回来看这里。
