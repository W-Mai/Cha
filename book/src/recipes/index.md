# Cookbook

Recipes for common situations. Each one starts with a problem statement and ends with a working set of commands or config.

| Recipe | When to read it |
|---|---|
| [Migrate from clippy](./migrate-clippy.md) | You have a Rust project on clippy and want Cha alongside or instead of it. |
| [CI on a monorepo](./monorepo-ci.md) | One repo, many packages, PRs only touch a few of them. |
| [Suppress in legacy code](./suppress-legacy.md) | You're adopting Cha mid-flight and CI is drowning in pre-existing findings. |
| [Custom plugin in 50 lines](./custom-plugin-50loc.md) | You want a project-specific detector, today. |
| [Calibrate to your codebase](./calibrate.md) | The defaults feel too strict or too loose for your code. |
| [Baseline workflow](./baseline.md) | Day-to-day rhythm of generating, comparing, and refreshing a baseline. |

If you're new, start with [CLI quick start](../quick-start/cli.md) and come back here once `cha analyze` is producing output.
