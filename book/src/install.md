# Install

## Shell (macOS / Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.sh | sh
```

## PowerShell (Windows)

```powershell
powershell -c "irm https://github.com/W-Mai/Cha/releases/latest/download/cha-cli-installer.ps1 | iex"
```

## Homebrew

```bash
brew install W-Mai/cellar/cha-cli
```

## From source

```bash
git clone https://github.com/W-Mai/Cha.git
cd Cha
cargo build --release
```

Requires [Rust](https://www.rust-lang.org/tools/install) (edition 2024).

## All platforms

Per-platform binaries (macOS aarch64 / x86_64, Linux musl + gnu, Windows) and SHA256 checksums are auto-synced from [cargo-dist](https://opensource.axo.dev/cargo-dist/) to the [Install page](/artifacts/).

## Verify the install

```bash
cha --version
cha analyze --help
```
