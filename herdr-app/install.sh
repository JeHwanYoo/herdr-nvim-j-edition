#!/usr/bin/env bash
# Installs herdr-nvim-j-edition into the Herdr app, with no layout
# edit: cmd-e opens the tab's nvim from an overlay, in the focused pane's
# folder, and the tree is the person's own nvim explorer (<leader>e).
#
#   herdr-app/install.sh [--home DIR] [--app-bin PATH] [--binary PATH] [--install-nvim]
#   herdr-app/install.sh --uninstall [--force] [--home DIR] [--app-bin PATH] [--binary PATH]
#
# The Herdr app must be open on that home (default: $HERDR_APP_HOME, else
# ~/.config/herdr-app). Install builds herdr-nvim with cargo (or takes the
# prebuilt --binary). It needs Neovim 0.10+: without one it asks to install it
# (install-nvim.sh: Homebrew's, else Neovim's release, no sudo), or, with no
# one to ask, stops unless --install-nvim says yes. Then it copies its runtime
# (binary + lua) to ${XDG_DATA_HOME:-~/.local/share}/herdr-nvim/app/<stamp>,
# copies the mods (herdr-nvim, herdr-nvim-picker) into <home>/mods with a
# wrapper at mods/herdr-nvim/bin/herdr-nvim (pointing at that runtime), runs
# `herdr-app check` on them and merges keys.json into the person's keys (only
# free keys: one they already use is kept and reported). An upstream
# (ChmaraX) install is replaced: its layout edits, its keys and its
# herdr-nvim-header mod are taken out. Run it again to upgrade in place: tab
# nvims already running keep running and are reported as stale; each restarts
# on the new runtime the next time it opens (unless it has unsaved work).
# Uninstall takes out the keys still bound to herdr-nvim, the mods and the
# runtime, and stops the app's nvims; one with unsaved buffers or pending
# comments is listed and kept (with its runtime) unless --force.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
repo=$(dirname "$here")
mode=install
home=${HERDR_APP_HOME:-$HOME/.config/herdr-app}
app_bin=${HERDR_APP_BIN:-}
binary=
install_nvim=
force=
mods=(herdr-nvim herdr-nvim-picker)
# Upstream's pane header mod: removed when it is herdr-nvim's own.
legacy=(herdr-nvim-header)

die() { echo "herdr-nvim: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case $1 in
    --uninstall) mode=uninstall ;;
    --force) force=1 ;;
    --home) home=$2; shift ;;
    --app-bin) app_bin=$2; shift ;;
    --binary) binary=$2; shift ;;
    --install-nvim) install_nvim=1 ;;
    -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
    *) die "unknown argument $1" ;;
  esac
  shift
done

# The herdr-app binary: given, on PATH, or a Herdr bundle (found by its bundle
# id, never by running an app that merely has the name).
find_app_bin() {
  if [ -n "$app_bin" ]; then echo "$app_bin"; return; fi
  if command -v herdr-app >/dev/null; then command -v herdr-app; return; fi
  for bundle in /Applications/*.app "$HOME"/Applications/*.app; do
    plist=$bundle/Contents/Info.plist
    [ -f "$plist" ] || continue
    [ "$(plutil -extract CFBundleIdentifier raw "$plist" 2>/dev/null)" = dev.herdr.app ] || continue
    echo "$bundle/Contents/MacOS/$(plutil -extract CFBundleExecutable raw "$plist")"
    return
  done
  die "cannot find the Herdr app; pass --app-bin /Applications/Herdr.app/Contents/MacOS/Herdr"
}

if [ -n "$binary" ]; then
  [ -x "$binary" ] || die "--binary $binary is not an executable"
  binary=$(cd "$(dirname "$binary")" && pwd)/$(basename "$binary")
fi

# The herdr-nvim binary to install: --binary, else this repo's release build.
built_binary() {
  if [ -n "$binary" ]; then echo "$binary"; return; fi
  local log
  log=$(mktemp)
  (cd "$repo" && cargo build --release --quiet 2>"$log") || { cat "$log" >&2; rm -f "$log"; die "cargo build failed"; }
  rm -f "$log"
  echo "$repo/target/release/herdr-nvim"
}

app_bin=$(find_app_bin)
home=$(cd "$home" 2>/dev/null && pwd) || die "no Herdr app home at $home"
export HERDR_APP_HOME=$home
"$app_bin" status >/dev/null 2>&1 || die "open the Herdr app first (home: $home)"

runtime=${XDG_DATA_HOME:-$HOME/.local/share}/herdr-nvim/app
wrapper=$home/mods/herdr-nvim/bin/herdr-nvim

# The picker was its own mod (herdr-nvim-pick) before it joined herdr-nvim as
# `herdr-nvim.pick`: drop that folder, when it is that mod.
retire_pick() {
  grep -qs '"nvim: open file from agent output"' "$home/mods/herdr-nvim-pick/mod.json" &&
    rm -rf "${home:?}/mods/herdr-nvim-pick"
  return 0
}

# Upstream's mods this fork doesn't have: dropped when they are herdr-nvim's.
retire_legacy() {
  local m
  for m in "${legacy[@]}"; do
    if ours "$m"; then rm -rf "${home:?}/mods/$m"; fi
  done
  return 0
}

# The runtime this install's wrapper runs (its plugin root), if there is one.
wrapper_root() {
  sed -n "s/^export HERDR_NVIM_PLUGIN_ROOT='\(.*\)'$/\1/p" "$wrapper" 2>/dev/null
}

# Whether a running process (a tab nvim: `--cmd set rtp+=<root>`) uses `root`.
in_use() {
  pgrep -f -- "set rtp\+=$1( |\$)" >/dev/null 2>&1
}

# Runtimes no running nvim uses, except `keep`: removed. The layout from
# before versioned runtimes ($runtime/{bin,lua,plugin}) counts as one.
prune_runtimes() {
  local keep=$1 dir
  for dir in "$runtime"/*/; do
    dir=${dir%/}
    [ -f "$dir/runtime-stamp" ] || continue
    [ "$dir" = "$keep" ] || in_use "$dir" || rm -rf "$dir"
  done
  if [ -d "$runtime/lua" ] && [ "$runtime" != "$keep" ] && ! in_use "$runtime"; then
    rm -rf "$runtime/bin" "$runtime/lua" "$runtime/plugin" "$runtime/runtime-stamp"
  fi
}

# herdr-nvim as the app's mods run it, from `root` (a runtime, or a build).
run_root() {
  local root=$1; shift
  HERDR_NVIM_HOST=app HERDR_NVIM_PLUGIN_ROOT="$root" HERDR_APP_BIN="$app_bin" HERDR_APP_HOME="$home" \
    "$root/bin/herdr-nvim" "$@"
}

# Whether mods/<m> is herdr-nvim's own: the wrapper this script writes is in
# it (herdr-nvim), or the mark file (the others). Any other folder of that
# name is someone else's and is never replaced.
MARK="# Written by herdr-nvim's herdr-app/install.sh"
ours() {
  grep -qsF "$MARK" "$home/mods/$1/bin/herdr-nvim" || grep -qsF "$MARK" "$home/mods/$1/.herdr-nvim"
}
mine_or_absent() {
  [ ! -e "$home/mods/$1" ] || ours "$1" ||
    die "$home/mods/$1 is not herdr-nvim's (no bin/herdr-nvim written by this script): move it away first, nothing changed"
}

if [ "$mode" = uninstall ]; then
  for m in "${mods[@]}"; do mine_or_absent "$m"; done
  # Through a runtime binary, not the wrapper: a half-replaced mod folder
  # must not keep the layouts and keys from being taken back. No runtime
  # left: this repo's own build.
  root=$(wrapper_root)
  [ -x "$root/bin/herdr-nvim" ] || root=$(ls -dt "$runtime"/*/ 2>/dev/null | head -1 | sed 's:/$::')
  [ -x "$root/bin/herdr-nvim" ] || root=$runtime
  if [ ! -x "$root/bin/herdr-nvim" ]; then
    built=$(built_binary)
    root=$(mktemp -d)
    mkdir -p "$root/bin" && cp "$built" "$root/bin/herdr-nvim"
    cp -R "$repo/lua" "$repo/plugin" "$root/"
  fi
  # The Files button back first, if `tree = "nvim"` took it out (the
  # controller does it, sync.js; the mods go below).
  "$app_bin" do herdr-nvim.uninstall >/dev/null 2>&1 && sleep 0.5 || true
  run_root "$root" app-setup uninstall --keys "$here/keys.json"
  # Unsaved work is never thrown away unasked: those nvims are listed and
  # kept, with the runtime they run.
  if ! stopped=$(run_root "$root" daemons stop --all ${force:+--force} 2>&1); then
    echo "$stopped" >&2
    echo "herdr-nvim: kept the nvims above; save (:wa) and quit them, or run --uninstall --force to discard their work" >&2
  elif [ -n "$stopped" ]; then
    echo "$stopped"
  fi
  hc kill herdr-nvim-watch >/dev/null 2>&1 || true
  for m in "${mods[@]}"; do rm -rf "${home:?}/mods/$m"; done
  retire_legacy
  retire_pick
  prune_runtimes ""
  rmdir "$runtime" 2>/dev/null || true
  echo "herdr-nvim: uninstalled from $home"
  exit 0
fi

for m in "${mods[@]}"; do mine_or_absent "$m"; done

built=$(built_binary)

# Neovim 0.10+, before anything changes: without it nothing would open.
if ! "$built" app-nvim-check >/dev/null 2>&1; then
  why=$("$built" app-nvim-check 2>&1 >/dev/null | sed 's/^herdr-nvim: //' || true)
  how="Homebrew's (brew install neovim)"
  command -v brew >/dev/null 2>&1 ||
    how="Neovim's own release, into ${XDG_DATA_HOME:-~/.local/share}/herdr-nvim/nvim (no sudo)"
  if [ -z "$install_nvim" ] && { : </dev/tty; } 2>/dev/null; then
    printf 'herdr-nvim: %s.\nInstall it now, %s? [Y/n] ' "$why" "$how" >/dev/tty
    read -r answer </dev/tty || answer=n
    case $answer in ''|y|Y|yes) install_nvim=1 ;; esac
  fi
  [ -n "$install_nvim" ] ||
    die "$why. Install it (or run this again with --install-nvim: $how); nothing changed"
  sh "$here/install-nvim.sh" --herdr-nvim "$built" || die "Neovim was not installed; nothing changed"
fi

# The runtime goes into its own folder, named by its stamp, and the wrapper
# points at it: a tab nvim already running keeps the runtime it started with,
# whole (never new Lua modules loaded beside old ones), and is stale when its
# runtime isn't the wrapper's (restarted when its pair next opens, never
# stopped here).
mkdir -p "$runtime"
staged=$(mktemp -d "$runtime/.new.XXXXXX")
mkdir -p "$staged/bin"
cp "$built" "$staged/bin/herdr-nvim"
cp "$here/install-nvim.sh" "$staged/bin/install-nvim.sh"
cp -R "$repo/lua" "$repo/plugin" "$staged/"
stamp=$(cd "$staged" && find bin lua plugin -type f | LC_ALL=C sort | xargs shasum -a 256 |
  shasum -a 256 | cut -c1-16)
echo "$stamp" > "$staged/runtime-stamp"
root=$runtime/$stamp
if [ -d "$root" ]; then
  # The same runtime again: only its stamp file is made right.
  mv -f "$staged/runtime-stamp" "$root/runtime-stamp"
  rm -rf "$staged"
else
  mv "$staged" "$root"
fi

# Nothing of the person's changes until the install is known to fit: the
# width and the layout herdr-nvim would edit, checked with the new runtime.
run_root "$root" app-setup check --keys "$here/keys.json" ||
  die "nothing changed"

# Each mod folder is made whole beside the live one (with its wrapper) and
# checked; only then are they swapped in by renames, and the old ones come
# back if a swap fails, so the app never loads half a copy (or one new mod
# beside an old one) and an uninstall always finds a runtime.
for m in "${mods[@]}"; do
  stage=$home/mods/.$m.new old=$home/mods/.$m.old
  new=$stage/$m # named as the mod, for the check
  rm -rf "$stage" "$old"
  mkdir -p "$stage"
  cp -R "$here/mods/$m" "$new"
  if [ "$m" = herdr-nvim ]; then
    mkdir -p "$new/bin"
    cat > "$new/bin/herdr-nvim" <<WRAPPER
#!/bin/sh
$MARK; run it again to change this.
export HERDR_NVIM_HOST=app
export HERDR_NVIM_SELF='$wrapper'
export HERDR_NVIM_PLUGIN_ROOT='$root'
export HERDR_APP_BIN='$app_bin'
export HERDR_APP_HOME='$home'
exec '$root/bin/herdr-nvim' "\$@"
WRAPPER
    chmod +x "$new/bin/herdr-nvim"
  else
    echo "$MARK" > "$new/.herdr-nvim"
  fi
  "$app_bin" check "$new" || {
    for n in "${mods[@]}"; do rm -rf "$home/mods/.$n.new"; done
    die "herdr-app check failed for $m; nothing replaced"
  }
done
# Every mod staged and checked: now swap them in, all or none.
swapped=()
for m in "${mods[@]}"; do
  stage=$home/mods/.$m.new old=$home/mods/.$m.old
  [ ! -e "$home/mods/$m" ] || mv "$home/mods/$m" "$old"
  if mv "$stage/$m" "$home/mods/$m"; then
    swapped+=("$m")
  else
    [ ! -e "$old" ] || mv "$old" "$home/mods/$m"
    for n in "${swapped[@]}"; do
      rm -rf "$home/mods/$n" && mv "$home/mods/.$n.old" "$home/mods/$n" 2>/dev/null
    done
    die "could not put $m in place; the previous mods are back"
  fi
done
for m in "${mods[@]}"; do rm -rf "$home/mods/.$m.old" "$home/mods/.$m.new"; done
retire_pick
prune_runtimes "$root"

"$wrapper" app-setup install --keys "$here/keys.json"
# After the layouts no longer name it (`app-setup install` took upstream's
# edits out).
retire_legacy
# The watcher that stops gone tabs' nvims runs the old runtime until it ends:
# end it; the next nvim pane starts it again on this one.
hc kill herdr-nvim-watch >/dev/null 2>&1 || true
echo "herdr-nvim: installed in $home"
