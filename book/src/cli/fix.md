# fix

Apply auto-fixes for the small set of findings that have a safe AST-level rewrite.

Today this means **`naming_convention` only**: classes whose names violate PascalCase are renamed via the `Plugin::try_fix` pathway, and every reference to the class is updated in step. The rewrite is AST-aware — identifiers inside string literals and comments are left alone.

No other detector ships an auto-fix yet. Running `fix` over a project where no `naming_convention` findings exist will report `Nothing to fix.` even if other smells are present.

## Usage

`cha fix [PATHS]... [FLAGS]`

Paths default to `.`.

## Examples

```bash
# Preview changes without writing
cha fix --dry-run

# Apply fixes to a directory
cha fix src/

# Only fix files changed in the working tree
cha fix --diff

# Combine: preview just the diff
cha fix --diff --dry-run
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--diff` | `false` | Only fix files changed in `git diff` (unstaged). |
| `--dry-run` | `false` | Print what would change without modifying any file. |

The global `--config <PATH>` flag also applies.

## Output

```
3 fix(es) applied.
```

In `--dry-run` mode the message becomes `... would be applied.` and the unified diff of the proposed changes is printed first.

## Caveats

- `cha fix` does not stage changes for git. Review with `git diff` and commit yourself.
- Renaming a class touches every reference Cha can resolve from its parsed model. Cross-language references (e.g. a Rust class name appearing in a TypeScript template) are out of scope.
- Always run on a clean working tree the first time.

## See also

- [analyze](./analyze.md) — surfaces every finding, fixable or not.
- [`naming` plugin](../plugins/reference.md) — the only smell with an auto-fix today.
