# 安装

## Shell（macOS / Linux）

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.sh | sh
```

## PowerShell（Windows）

```powershell
powershell -c "irm https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.ps1 | iex"
```

## Homebrew

```bash
brew install W-Mai/cellar/cha-cli
```

## 从源码

```bash
git clone https://github.com/W-Mai/Cha.git
cd Cha
cargo build --release
```

需要 [Rust](https://www.rust-lang.org/tools/install)（edition 2024）。

## 完整平台清单

每个平台的二进制（含 macOS aarch64 / x86_64、Linux musl + gnu、Windows）和 SHA256 校验和由 [cargo-dist](https://opensource.axo.dev/cargo-dist/) 自动同步生成，落在 [Install 页](/artifacts/) —— 该页是英文，但下载链接和命令都是通用的。

## 验证

```bash
cha --version
cha analyze --help
```
