# preset

Inspect the builtin language profiles. Each supported language ships with a default
set of enabled / disabled plugins; `preset` reports what would run for that language
under the current `.cha.toml`.

## Usage

```
cha preset list
cha preset show <language>
```

`<language>` is one of `rust`, `typescript`, `python`, `go`, `c`, `cpp`.

## Examples

```bash
# List supported languages and the count of rules disabled by each builtin profile.
cha preset list

# Show the resolved plugin set, strictness factor, and disabled smells for C.
cha preset show c

# Same for Rust.
cha preset show rust
```

## Subcommands

### `list`

Prints one line per supported language with the number of rules the builtin profile
disables by default. C is the only language that disables rules out of the box
(`naming`, `lazy_class`, `data_class`, `design_pattern`).

### `show <language>`

Resolves the configuration for the given language and prints:

- The current strictness factor (`relaxed` = 2.0×, `default` = 1.0×, `strict` = 0.5×,
  or a custom float).
- Every plugin that would run, with its description.
- Any plugins disabled by the builtin profile.
- Any smells disabled via `[languages.<lang>] disabled_smells = [...]`.

The output ends with a reminder of the override syntax:

```
[languages.<lang>.plugins.<name>]
enabled = true
```

## Flags

`preset` itself takes no flags; the global `--config <path>` works as usual.

## See also

- [Strictness and presets](../configuration/presets.md)
- [Configuration keys](../reference/config-keys.md)
- [Smells reference](../plugins/reference.md)
