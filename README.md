# herdr-nvim-j-edition

A fork of [ChmaraX/herdr-nvim](https://github.com/ChmaraX/herdr-nvim) with a
few changes to fit my own workflow.

Each tab of the [Herdr](https://herdr.dev) app gets its own persistent nvim,
one key away. You can open the files your agent just touched, and send the
comments you write in nvim back to the agent.

**Who it's for:** if you'd rather browse files in your own nvim setup, such
as LazyVim's explorer and other TUI tools, than in the Herdr app's file tree
panel, this fork is worth a try. It leaves your layout alone and lets your
nvim config handle the file tree.

## What's different from upstream

| | Upstream | This fork |
| --- | --- | --- |
| Layout | The install edits your layout (the Files slot, a controller block, pane headers) and switches to a copy of Herdr's built-in layout | Never touches a layout. `cmd-e` is an overlay mod that toggles the tab's nvim pane |
| File tree | A native tree panel; takes over `<leader>e` | Your nvim's own explorer (LazyVim: `<leader>e` opens Snacks' explorer) |
| Folder nvim opens in | The git root of the folder the tab works in | **The pwd of the focused pane when you press `cmd-e`.** An nvim that's already running is `:cd`'d there on every open |
| Preview | Markdown, image and mermaid preview pane | None. Images open in nvim too |
| `cmd-o` | `herdr-nvim.pick`, a controller command | `herdr-nvim-picker`, its own overlay mod |

These still work as in upstream: one nvim per tab, with buffers, cursor and
language servers kept when its pane is closed; the file picker; and comments
sent to the agent.

## Requirements

- Herdr app 0.1.11+ (tested on macOS; Linux uses the same scripts but is untested)
- Neovim 0.10+
- A Rust toolchain (cargo). This fork ships no release binaries, so it builds from source.

## Install

With the Herdr app open:

```sh
git clone https://github.com/JeHwanYoo/herdr-nvim-j-edition.git
cd herdr-nvim-j-edition
herdr-app/install.sh              # install; run it again to upgrade
herdr-app/install.sh --uninstall  # remove it
```

The installer does three things:

1. builds the binary and copies it, with its Lua runtime, into `~/.local/share/herdr-nvim/app`;
2. adds two mods, `herdr-nvim` and `herdr-nvim-picker`;
3. adds its keys to `keys.json`, but only where the key is free.

If upstream herdr-nvim is installed, the installer replaces it. It removes
upstream's layout edits, its keys (`herdr-nvim.pick`, `herdr-nvim.reveal`)
and its `herdr-nvim-header` mod.

## Keys

| Key | Does |
| --- | --- |
| `cmd-e` | Open or close the tab's nvim, in the focused pane's pwd |
| `cmd-o` | Pick a file the agent read or edited, and open it in nvim |
| `<leader>e` (in nvim) | Your own explorer, as your config maps it |

## File picker

`cmd-o` reads the tab's agent's session log (pi or Claude Code). It lists the
files the agent touched, newest first, followed by uncommitted git changes.
Without a log, it uses the paths shown on the agent's screen. Type to
fuzzy-search the whole repo, then press `⏎` to open the file in nvim.

## Comments

| Keymap | Command | Action |
| --- | --- | --- |
| `<leader>ac` | `:Herdr comment` | Comment on the current line or selection |
| `<leader>al` | `:Herdr list` | List comments (`⏎` edit, `d` delete) |
| `<leader>as` | `:Herdr send` | Paste all comments into the agent's input |
| `<leader>aS` | `:Herdr submit` | Send all comments to the agent |
| `<leader>ai` | `:Herdr ref` | Insert `path:line` at the agent's cursor |

Comments go to the tab's agent. If the tab has none, they go to the space's
only agent. If there are several, nvim asks which one.

## Config

`~/.config/herdr-nvim/config.toml` (optional):

```toml
[sidebar]
nvim_bin = "nvim"       # the Neovim to run
nvim_env = []           # e.g. ["NVIM_APPNAME=myapp"]
position = "right"      # or "bottom"
```

This fork ignores upstream's `[app] tree` and `tree_key` settings.

## Troubleshooting

- To list each tab's running nvim and its memory use, run `~/.config/herdr-app/mods/herdr-nvim/bin/herdr-nvim daemons`.
- Closing a tab stops its nvim and language servers, and unsaved buffers are lost.
- After an upgrade, nvims that are already running stay on the old version.
  Close and reopen one with `cmd-e` to restart it on the new version. An
  nvim with unsaved buffers is kept as it is.

## Development

```sh
just ci   # cargo fmt --check + cargo test + headless Lua tests
```

The Herdr TUI (terminal) plugin is unchanged from upstream: see [TUI.md](TUI.md).
How the mods are built: [herdr-app/RECIPE.md](herdr-app/RECIPE.md).

## License

MIT. [LICENSE](LICENSE) keeps the original copyright (Adam Chmara) and adds
one for this fork's changes.
