import { div } from "gpui-kit";
import { TextView, v_flex } from "gpui-base";
import { exec, settings, theme, watchFolder } from "herdr";
import { nvim } from "./lib.js";

export class Preview {
  constructor({ preview, tab, space, cwd }, cx) {
    this.source = { tab, space, cwd };
    const folder = preview.slice(0, preview.lastIndexOf("/")) || "/";
    cx.spawn(async (cx) => {
      while (!this.closed) {
        const result = await exec("cat", [preview]).catch(() => null);
        if (this.closed) break;
        this.text = result?.code === 0 ? result.stdout : null;
        cx.notify();
        await watchFolder(folder).catch(() => this.close());
      }
    });
  }

  close() {
    this.closed = true;
  }

  render() {
    const colors = theme();
    const size = settings().font_size;
    const body = typeof this.text === "string"
      ? TextView.markdown("herdr-nvim-preview-text", this.text).selectable(true)
      : div().child(this.text === null ? "Could not read the file." : "Loading preview…");
    return v_flex()
      .size_full()
      .bg(colors.background)
      .text_size(typeof size === "number" ? size : 14)
      .text_color(colors.foreground)
      .child(div().id("herdr-nvim-preview-source").px(12).py(6).cursor_pointer()
        .on_click(() => { void nvim("toggle", ["--state", "open"], this.source); })
        .child("Source"))
      .child(v_flex().id("herdr-nvim-preview-scroll").flex_1().min_h(0)
        .px(18).py(12).overflow_y_scrollbar().child(body));
  }
}
