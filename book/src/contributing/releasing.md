# Releasing

The release process is a single command — `cargo xtask release` — that pushes, waits for CI, tags, waits for the release workflow, then publishes to crates.io. This page documents the invariants you need to honour before pressing the button, and what to do when something goes wrong mid-flight.

## Pre-release checklist

Before `cargo xtask release`:

- [ ] Working tree is clean (`git status --short` empty). The release script refuses to run otherwise.
- [ ] You're on `main`, up to date with `origin/main`.
- [ ] `CHANGELOG.md` has an `[Unreleased]` section that describes the changes since the last tag. The release script does not auto-generate notes — what's in the file is what ships.
- [ ] `cargo xtask ci` passes locally.
- [ ] `cargo xtask analyze` passes (Cha self-analysis).

Then bump:

```bash
cargo xtask bump <major|minor|patch>
```

This rewrites `version` in every workspace `Cargo.toml`, refreshes every `Cargo.lock`, and syncs `vscode-cha/package.json`. **Do this in its own commit**:

```bash
git add -p
git commit -m "🔖: bump version to x.y.z"
```

`bump` does not move `[Unreleased]` notes to a new versioned section in `CHANGELOG.md`. Move them by hand in the same commit:

```diff
 ## [Unreleased]

+## [1.20.0] - 2026-06-04
+
 ### Added
 - ...
```

The new section needs a date and the version. Empty `[Unreleased]` stays at the top.

## Release

```bash
cargo xtask release
```

What it does, in order:

1. Verifies the working tree is clean.
2. Reads the workspace version, computes the tag (`v<version>`).
3. `git push origin main`.
4. Waits up to 20 minutes for `ci.yml` to pass on the pushed sha.
5. Creates and pushes the tag (`v<version>`).
6. Waits up to 30 minutes for `release.yml` to complete (it runs cargo-dist to produce installers and platform binaries, attaches them to the GitHub release).
7. Runs `cargo publish` for every crate, in dependency order.

The script is idempotent up to the first failed step. If CI fails on step 4, fix the failure and re-run from scratch — nothing has been tagged yet.

## Mid-flight failures

**CI fails after push (step 4):**
The push happened, but no tag exists yet. Push the fix, re-run `cargo xtask release`. The script picks up the same version and waits for the new CI run.

**Release workflow fails (step 6):**
Tag exists. The release.yml workflow can be re-run from the GitHub UI (`gh run rerun <id>`). If the failure is in your code, you need a new version — bump again, redo from step 1. **Never re-tag the same version**; cargo-dist's installers fingerprint the release and will silently break.

**`cargo publish` fails (step 7):**
Some crates may have published already, others not. Re-run `cargo xtask publish` (without `release`) to retry. crates.io idempotently rejects already-published versions, so this is safe.

## What `release.yml` produces

- Platform binaries: macOS aarch64/x86_64, Linux aarch64/x86_64 (musl + gnu), Windows x86_64.
- Installers: shell (`cha-cli-installer.sh`), PowerShell (`cha-cli-installer.ps1`), Homebrew tap entry.
- Release notes: extracted from `CHANGELOG.md` for the current version.
- All artifacts attached to the GitHub release at `v<version>`.

## After release

- `cargo xtask release` writes nothing to your local tree beyond the tag. No follow-up commits needed.
- Verify the [Marketplace listing](https://marketplace.visualstudio.com/items?itemName=BenignX.vscode-cha) updates within ~10 minutes. If not, check the `vscode-publish.yml` workflow — it auto-publishes on tag.
- Bump the example versions in README and pre-commit/GitHub Action snippets if any users will copy them. Currently those reference `v1.19.0`; if you released `v1.20.0`, update or accept that users will lag.

## Yanking

If a release breaks something critical:

```bash
cargo yank --version 1.20.0 cha-cli
cargo yank --version 1.20.0 cha-core
# ... for every crate
```

Yanking does not delete the crate; it prevents new dependents from picking it up. The GitHub release tag and binaries remain. Cut a `1.20.1` with the fix.

## See also

- [`xtask/src/release.rs`](https://github.com/W-Mai/Cha/blob/main/xtask/src/release.rs) — the actual implementation.
- [`xtask/src/main.rs`](https://github.com/W-Mai/Cha/blob/main/xtask/src/main.rs) — `cmd_bump`, `cmd_publish`.
- [`.github/workflows/release.yml`](https://github.com/W-Mai/Cha/blob/main/.github/workflows/release.yml)
