// herdr-nvim-j-edition for the Herdr app: the tab's nvim, a real
// nvim terminal in a full-height pane of the tab, backed by that tab's
// persistent daemon. No layout edit, no side panel: the file tree is the
// person's own nvim explorer (`<leader>e` in their config).
//
// One mod, four roles, chosen by its props:
// - an overlay, `mod.open:herdr-nvim` (cmd-e): opens or closes the tab's nvim
//   pane (`herdr-nvim toggle`), then closes. Opening `cd`s the tab's nvim to
//   the focused pane's folder;
// - a dialog with props {card}: the welcome and "Neovim is needed" (card.js);
// - a pane with props {preview}: the saved Markdown file (preview.js);
// - a block with props {bar}: the title of cmd-o's picker (pick.js, opened by
//   the `herdr-nvim-picker` mod).

import { View, div } from "gpui-kit";
import { closeOverlay, props } from "herdr";
import { nvim, place } from "./lib.js";
import { Card, welcome } from "./card.js";
import { PickBar } from "./pick.js";
import { Preview } from "./preview.js";

export default class HerdrNvim extends View {
  init(_props, cx) {
    const options = props() ?? {};
    const { overlay, bar, card, preview } = options;
    if (preview) this.part = new Preview(options, cx);
    else if (bar) this.part = new PickBar();
    else if (card) this.part = new Card(cx);
    else if (overlay) {
      // The tab and the focused pane's folder as they are at the press,
      // before any await.
      const at = place();
      cx.spawn(async () => {
        try {
          const out = await nvim("toggle", [], at);
          let made = {};
          try { made = JSON.parse(out || "{}"); } catch {}
          // The first nvim pane ever: its keys, once.
          if (made.terminal) welcome();
        } finally {
          closeOverlay(overlay);
        }
      });
    }
  }

  render(cx) {
    return this.part ? this.part.render(cx) : div();
  }

  close() {
    this.part?.close?.();
  }
}
