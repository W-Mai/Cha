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

完整平台和单独二进制见 [cha.to01.icu](https://cha.to01.icu)。

## 验证

```bash
cha --version
cha analyze --help
```
