# completions

Generate shell completion scripts. Cha uses `clap_complete`'s dynamic engine, so
completions resolve plugin names live by reading `.cha/plugins/` and the merged
plugin registry — newly installed plugins become tab-completable without
regenerating the script.

## Usage

```
cha completions [shell]
```

`shell` is one of `bash`, `zsh`, `fish`, `powershell`, `elvish`. Running `cha
completions` with no argument prints quick-start instructions instead of a script.

## Examples

```bash
# Print setup hints for every supported shell.
cha completions

# Source completions for the current session.
eval "$(cha completions bash)"
eval "$(cha completions zsh)"
cha completions fish | source

# Install permanently.
cha completions fish > ~/.config/fish/completions/cha.fish
cha completions zsh  > ~/.local/share/zsh/site-functions/_cha
```

## Dynamic plugin names

The `--plugin` flag of `cha analyze` is annotated with a dynamic candidate
provider. After completions are installed, typing

```bash
cha analyze --plugin <TAB>
```

lists every built-in plugin plus every WASM plugin currently in `.cha/plugins/` or
`~/.cha/plugins/`, each with its description as the completion help text. No
regeneration needed when you `cha plugin install` a new one.

Powered shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`. Other shells fall
back to static completions automatically.

## Flags

`completions` takes no flags beyond the shell argument.

## See also

- [`plugin`](./plugin.md) — manage plugins (newly installed names show up in
  completions immediately).
- [`analyze`](./analyze.md) — the main consumer of dynamic plugin completion.
