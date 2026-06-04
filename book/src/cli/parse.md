# parse

Show what Cha sees after parsing a file: language, line count, functions, classes, and imports.

Useful for debugging detector behaviour ("why didn't `long_method` fire?"), confirming that a file is recognised as the right language, and inspecting the structural data plugins receive.

## Usage

`cha parse [PATHS]...`

Paths default to `.`. All supported languages are parsed; unsupported files are silently skipped.

## Examples

```bash
# Inspect a directory
cha parse src/

# Inspect one file
cha parse src/main.rs

# Quickly see how big the parsed model is for the whole project
cha parse | wc -l
```

A typical line of output:

```
=== src/main.rs (rust) ===
  lines: 312
  functions: 18
    - run_analysis (L62-L98, 37 lines, complexity 6)
    - print_report (L120-L155, 36 lines, complexity 4)
  classes: 2
    - AnalyzeOpts (L20-L28, 0 methods, 9 lines)
  imports: 4
    - cha_core (L3)
    - rayon::prelude::* (L5)
```

The `lines`, `complexity`, `method_count`, etc. shown here are the same numbers detectors threshold against.

## Flags

`parse` has no flags beyond the global `--config <PATH>`.

## See also

- [analyze](./analyze.md) — run detectors over the same parsed model.
- [Smells reference](../plugins/reference.md) — which fields each detector reads.
