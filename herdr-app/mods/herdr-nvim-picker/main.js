// cmd-o (`mod.open:herdr-nvim-picker`): a file from the agent's output,
// opened in the tab's nvim. A mod of its own because keys reach a mod only
// through `mod.open` when no copy of it is on screen (this fork puts no
// block in the layout). A mod can't import another mod's files, so this is
// pick.js's flow again: `herdr-nvim pick-file` gathers the candidates and
// starts the picker in a hidden core terminal, shown in a dialog under
// herdr-nvim's title bar ({bar}); ⏎ opens the file in the tab's nvim.

import { View, div } from "gpui-kit";
import { appHome, closeOverlay, exec, focused, notify, openOverlay, props } from "herdr";

function place(f = focused()) {
  if (!f?.space || !f?.tab) return null;
  const terminal = f.terminal ?? null;
  return { space: f.space, tab: f.tab, terminal, cwd: terminal ? f.cwd ?? null : null };
}

export default class HerdrNvimPicker extends View {
  init(_props, cx) {
    const { overlay } = props() ?? {};
    const at = place();
    cx.spawn(async () => {
      let result = null;
      try {
        if (!at) {
          notify({ title: "herdr-nvim", body: "Open a tab first.", level: "warning" });
          return;
        }
        const bin = `${appHome()}/mods/herdr-nvim/bin/herdr-nvim`;
        const r = await exec(bin, ["pick-file", "--ctx", JSON.stringify(at)]);
        if (r.code !== 0) {
          notify({ title: "herdr-nvim pick-file failed", body: (r.stderr || r.stdout).trim().slice(-400), level: "error" });
          return;
        }
        result = JSON.parse(r.stdout);
      } catch (e) {
        notify({ title: "herdr-nvim is not installed here", body: String(e), level: "error" });
      } finally {
        if (overlay) closeOverlay(overlay);
      }
      if (!result) return;
      if (result.empty) {
        notify({ title: "herdr-nvim: nothing to pick", body: result.empty, level: "warning", timeout: 8000 });
        return;
      }
      openOverlay({
        kind: "dialog",
        modal: true,
        width: 820,
        height: 470,
        layout: { type: "column", children: [
          { type: "block", size: { px: 38 }, content: { mod: "herdr-nvim", props: { bar: true } } },
          { type: "block", content: { terminal: result.terminal } },
        ] },
      });
    });
  }

  render() {
    return div();
  }
}
