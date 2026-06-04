# 发版

发版流程是一条命令 —— `cargo xtask release`。它会推、等 CI、打 tag、等 release workflow，最后发到 crates.io。这一篇写两件事：按这条命令前要确认的几条不变量；中途出错时要怎么处理。

## 发版前 checklist

跑 `cargo xtask release` 之前：

- [ ] 工作树干净（`git status --short` 没有输出）。release 脚本不干净就不跑。
- [ ] 你在 `main`，且与 `origin/main` 同步。
- [ ] `CHANGELOG.md` 里的 `[Unreleased]` 已经写好这次的变更内容。release 脚本不会自动生成 release notes —— 文件里写什么、release 出什么。
- [ ] `cargo xtask ci` 本地跑过。
- [ ] `cargo xtask analyze` 通过（cha 自分析）。

然后 bump：

```bash
cargo xtask bump <major|minor|patch>
```

这会改 workspace 里所有 `Cargo.toml` 的 `version`，刷新所有 `Cargo.lock`，同步 `vscode-cha/package.json` 的版本号。**单独一个 commit**：

```bash
git add -p             # -p 是 patch 模式，让你逐 hunk 选择要 stage 的内容；只挑 bump 相关的几行
git commit -m "🔖: bump version to x.y.z"
```

`bump` 不会把 `[Unreleased]` 内容自动搬到一个版本号 section。你要在同一个 commit 里手动搬：

```diff
 ## [Unreleased]

+## [1.20.0] - 2026-06-04
+
 ### Added
 - ...
```

新 section 要带日期和版本号，空的 `[Unreleased]` 留在最上面。

## 发版

```bash
cargo xtask release
```

它做的事情，按顺序：

1. 检查工作树是否干净。
2. 读 workspace 版本，算出 tag（`v<version>`）。
3. `git push origin main`。
4. 等 `ci.yml` 在这次 push 的 sha 上通过，最多 20 分钟。
5. 创建并推 tag（`v<version>`）。
6. 等 `release.yml` 跑完，最多 30 分钟（它跑 cargo-dist 出各平台二进制和 installer，再挂到 GitHub release 上）。
7. 按依赖顺序对每个 crate 跑 `cargo publish`。

脚本是分步幂等的：前面任何一步挂掉，修完都可以从头再跑。第 4 步 CI 挂了的话 tag 还没打，修完再跑一次 `cargo xtask release` 就行——版本不变，等下一次 CI 通过即可。

## 中途出错怎么办

**push 之后 CI 挂了（第 4 步）：**
push 已经发生，但 tag 还没打。修完再 `cargo xtask release`。

**release workflow 挂了（第 6 步）：**
tag 已经存在。release.yml 可以从 GitHub UI 重新跑（`gh run rerun <id>`）。如果挂的是你的代码问题，需要切一个新版本 —— 重新 bump、从第 1 步重来。**绝对不要在同一个版本号上重打 tag**：cargo-dist 的 installer 把版本号写死在脚本里，重打 tag 会让已经下载过 installer 的用户和现在的产物对不上号、静默坏掉。

**`cargo publish` 挂了（第 7 步）：**
有些 crate 可能已经发出去了，有些没。再跑 `cargo xtask publish`（不带 release）就行。crates.io 对已发布的版本会直接拒绝、不会重复发布，所以重跑是安全的。

## `release.yml` 产出什么

- 各平台二进制：macOS aarch64 / x86_64、Linux aarch64 / x86_64（musl + gnu）、Windows x86_64。
- Installer：shell（`cha-cli-installer.sh`）、PowerShell（`cha-cli-installer.ps1`）、Homebrew tap entry。
- Release notes：从 `CHANGELOG.md` 抽出本版本对应内容。
- 所有产物挂在 `v<version>` 这个 GitHub release 上。

## 发版后

- `cargo xtask release` 在本地除了打那个 tag 之外不写任何东西，不需要追加 commit。
- 看一眼 [Marketplace](https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha)，10 分钟左右会更新。如果没动，去 `vscode-publish.yml` workflow 看日志 —— 它是在 tag 触发时自动发的。
- README、pre-commit / GitHub Action 片段里写死的版本号要不要更新自己定。当前那些写的是 `v1.19.0`；如果你出了 `v1.20.0`，要么改文档，要么接受用户拷过去的版本号会暂时落后一档。

## Yank

如果一个版本被发现有严重问题：

```bash
cargo yank --version 1.20.0 cha-cli
cargo yank --version 1.20.0 cha-core
# ... 每个 crate 都来一遍
```

yank 不是删除，crate 还在；只是 cargo 在解析依赖时不会再选中这个版本。GitHub release 的 tag 和二进制都还在。修完之后切个 `1.20.1` 出去。

## See also

- [`xtask/src/release.rs`](https://github.com/W-Mai/Cha/blob/main/xtask/src/release.rs) —— 真正的实现。
- [`xtask/src/main.rs`](https://github.com/W-Mai/Cha/blob/main/xtask/src/main.rs) —— `cmd_bump`、`cmd_publish`。
- [`.github/workflows/release.yml`](https://github.com/W-Mai/Cha/blob/main/.github/workflows/release.yml)
