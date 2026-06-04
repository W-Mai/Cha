# completions

生成 shell 补全脚本。装上之后 `cha <Tab>` 自动补命令、参数，**还能补已装插件的名字**（`--plugin <Tab>` 会列出当前 `.cha/plugins/` 和 `~/.cha/plugins/` 下的插件）。

## 用法

```
cha completions <shell>
```

支持：`bash` / `zsh` / `fish` / `powershell` / `elvish`。

## 示例

```bash
# fish
cha completions fish > ~/.config/fish/completions/cha.fish

# zsh（用户级）
cha completions zsh > ~/.local/share/zsh/site-functions/_cha
# 或者 oh-my-zsh：放进 ~/.oh-my-zsh/custom/

# bash
cha completions bash > ~/.local/share/bash-completion/completions/cha

# 不带参数：打印简短指引，告诉你应该把脚本放哪
cha completions
```

下一次起 shell 就生效。

## 参数

| 参数 | 默认 | 说明 |
|------|------|------|
| shell | — | 不带参数时只打印帮助；带 `bash`/`zsh`/`fish`/`powershell`/`elvish` 输出对应脚本 |

## 参考

- [`cha plugin list`](./plugin.md) —— 看动态补全能补出哪些插件名
