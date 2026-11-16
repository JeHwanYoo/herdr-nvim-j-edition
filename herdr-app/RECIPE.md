# herdr-nvim-j-edition

Neovim beside your agents, with no layout edit. `cmd-e` opens or closes the
tab's own nvim in a full-height pane, in the folder of the pane that had the
focus. The file tree is the person's own nvim explorer (`<leader>e`).
`cmd-o` lists the files the agent just read and edited, and comments written
in nvim go to the agent with their `file:line`.

Forked from [ChmaraX/herdr-nvim](https://github.com/ChmaraX/herdr-nvim).

## Adding it (for the app's agent)

Don't copy the folders: herdr-nvim needs its binary. With the app open, from
a clone:

```sh
herdr-app/install.sh              # builds with cargo; again to upgrade
herdr-app/install.sh --uninstall
```

Tell the person first: it builds `herdr-nvim` and copies it with its Lua into
`~/.local/share/herdr-nvim/app`; the mods run programs (an nvim per tab,
`hc`, `git`) and read the tab's repo and the agent's session logs, and use no
network; it adds two mods (`herdr-nvim`, `herdr-nvim-picker`) and its keys
where they are free. It never edits a layout; an upstream install's layout
edits, keys and `herdr-nvim-header` mod are taken out. Needs Neovim 0.10+.

## What you see

- **nvim pane**, right of the tab's other panes, as tall as the tab (below
  them with `[sidebar] position = "bottom"`). Its header is the app's own.
- **Picker** (`cmd-o`): an 820×470 dialog, "Open file from agent output" on
  a 38px `chrome` bar with its keys as keycaps, over a terminal picker: the
  agent's files newest first, then uncommitted git changes; typing
  fuzzy-searches the whole repo.
- **Cards**: "nvim is ready" once after the first nvim pane; without Neovim,
  "Neovim 0.10 or newer is needed" with **Not now** / **Install Neovim**.

## How it behaves

- **`cmd-e`**: in a tab without its nvim pane, opens it (the pane takes the
  keyboard); with it, closes it. Closing never stops the tab's nvim (a
  headless daemon per tab): the next open shows the same buffers, cursor and
  language servers.
- **Folder**: opening runs nvim in the focused pane's own folder, exactly
  (not its git root), and `:cd`s an already running tab nvim there. A
  focused mod pane has no folder: the tab's first terminal's is used.
- **Tree**: none of herdr-nvim's own. The person's explorer keys stay as
  their config maps them; nothing opens an explorer by itself.
- **Picker**: reads the tab's agent's session log (pi or Claude Code), then
  git changes; with no log, the paths on its screen. ⏎ opens the file in the
  tab's nvim (opening the pane if needed), Esc closes.
- **Comments** (`<leader>a…`): `ac` comment, `al` list, `as` paste, `aS`
  send, `ai` insert `path:line`; to the tab's agent, else nvim asks.
- **Closing** a tab stops its nvim and its language servers.

## Built from

- **The `herdr-nvim` binary** (Rust and Lua), run through the wrapper
  `mods/herdr-nvim/bin/herdr-nvim` (`HERDR_NVIM_HOST=app`). `toggle` opens
  the pane (`pane.split` across the tab's root, labelled
  `herdr-nvim.sidebar=<tab>`, running `herdr-nvim sidebar --cwd <folder>`,
  which `:cd`s the daemon and attaches `nvim --remote-ui`) or closes it;
  `pick-file` starts the picker in a hidden core terminal and prints
  `{"terminal": "tN"}` or `{"empty": "<why>"}`. Each takes `--ctx` (space,
  tab, focused terminal and its cwd) and exits 3 with `{"nvim_missing": …}`
  without a usable Neovim.
- **Mod `herdr-nvim`** (`place: none`): as the overlay `mod.open:herdr-nvim`
  (a 1×1 popup) it runs `toggle` for the tab captured at the press, then
  closes; with props `{card}` the cards; with `{bar}` the picker's title bar.
- **Mod `herdr-nvim-picker`** (`place: none`): as the overlay
  `mod.open:herdr-nvim-picker` it runs `pick-file`, closes, and opens the
  picker dialog (a mod can't import another mod's files, so the flow is
  repeated here).
- **Keys** (merged into `global` where free): `cmd-e` →
  `mod.open:herdr-nvim`, `cmd-o` → `mod.open:herdr-nvim-picker`. Install
  and uninstall remove keys bound to retired actions (`setup.rs` `RETIRED`,
  upstream's `herdr-nvim.pick` and `herdr-nvim.reveal` among them).

## Watch out

- Keys reach a mod's `nextCommand()` only while a copy is on screen; with
  no block in the layout, every key is a `mod.open` of an overlay mod.
- Capture `focused()` synchronously in the overlay's `init`: the tab and
  the pane's folder as they were at the press.
- A process started from a terminal that keeps using `hc` after the
  terminal ends must drop `HC_SCOPE` and `HC_TERMINAL_ID`.
- Local spaces only. Tested on macOS.

## Done when

- `herdr-app check` passes for both mods, `herdr-app status` shows no
  errors, and no layout file names `herdr-nvim`.
- `cmd-e` in a shell at `repo/src/app` opens nvim with `getcwd()` =
  `repo/src/app`; `cmd-e` closes it; `cd` elsewhere, `cmd-e` again: the same
  nvim, now in the new folder.
- In LazyVim, `<leader>e` in the tab's nvim opens Snacks' explorer.
- `cmd-o` over an agent that touched files lists them; ⏎ opens one in nvim.
- Closing the tab stops its nvim (`herdr-nvim daemons` no longer lists it).
