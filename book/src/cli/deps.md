# deps

Render a dependency graph of the project: imports between files, classes within files, or the function call graph.

The same parsed model that powers `cha analyze` feeds the graph builder, so call-graph edges are AST-resolved (not regex-matched) and import edges follow the same relative-path resolution rules as the detectors.

## Usage

`cha deps [PATHS]... [FLAGS]`

Paths default to `.`.

## Graph types

`--type` selects what the nodes and edges represent:

| `--type` | Nodes | Edges |
|----------|-------|-------|
| `imports` _(default)_ | Files (or directories with `--depth dir`) | Import / `use` / `#include` statements. |
| `classes` | Classes / structs / traits | Field types, method parameter types, inheritance. |
| `calls` | Functions | Static call edges resolved through the project index. |

## Examples

```bash
# Default DOT output of file-level imports
cha deps --format dot

# Aggregate to directory level, render via Mermaid
cha deps --format mermaid --depth dir

# Class diagram with fields and methods, filtered to anything matching "Plugin"
cha deps --type classes --filter Plugin --detail --format plantuml

# Who calls the function `analyze`?
cha deps --type calls --filter analyze --direction in

# What does `analyze` call?
cha deps --type calls --filter analyze --direction out

# Dependency Structure Matrix
cha deps --format dsm

# Single-file HTML viewer
cha deps --format html --output deps.html

# Pipe DOT into Graphviz
cha deps --format dot | dot -Tsvg > deps.svg
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--type <KIND>` | `imports` | `imports`, `classes`, or `calls`. |
| `--format <FMT>` | `dot` | Output format: `dot`, `json`, `mermaid`, `plantuml`, `dsm`, `terminal`, `html`. |
| `--depth <D>` | `file` | Aggregation: `file` (one node per file) or `dir` (one node per directory). `dir` only applies to `--type imports`. |
| `--filter <REGEX>` | _(none)_ | Keep only nodes whose name matches the regex, plus their connected subgraph. |
| `--exact` | `false` | With `--filter`, only show edges whose endpoints directly match — no transitive expansion. |
| `--detail` | `false` | For `--type classes`, render fields and methods inside each node. |
| `--direction <DIR>` | `both` | For `--type calls` with `--filter`: `in` (callers of the matched node), `out` (callees), or `both`. |

The global `--config <PATH>` flag also applies; `exclude` patterns are honoured.

## Output

`dot` and `json` are stable formats meant for piping into other tools. `mermaid` and `plantuml` paste straight into a Markdown / wiki page. `dsm` renders a triangular matrix that makes layering and cycles visible at a glance. `terminal` is a compact text rendering for quick inspection. `html` writes a self-contained interactive page (use with `--output`).

## See also

- [layers](./layers.md) — infer architectural layers from the same import graph.
- [Migrate from clippy recipe](../recipes/migrate-clippy.md)
