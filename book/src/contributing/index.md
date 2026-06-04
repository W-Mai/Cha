# Contributing

Three things that are useful to know before sending a PR. None of them are mandatory reading — the project is small enough to find your way by reading code — but each saves a round-trip in review.

| Page | Read it before |
|---|---|
| [Architecture](./architecture.md) | Touching anything that crosses crate boundaries (`cha-core` ↔ `cha-parser`, plugin runtime, LSP). |
| [Writing a smell](./writing-a-smell.md) | Adding a new built-in detector (`cha-core/src/plugins/`). |
| [Releasing](./releasing.md) | Cutting a release. Mostly automated; a few invariants exist. |

Local dev:

```bash
cargo xtask ci          # everything CI runs
cargo xtask test        # tests only
cargo xtask lint        # clippy + fmt
cargo xtask analyze     # cha self-analyses every output format
```

Code review style: small commits, separate concerns. Bug fix in one commit, refactor in the next, doc update in a third. The [release script](./releasing.md) (`cargo xtask release`) squashes nothing — clean history is on you.
